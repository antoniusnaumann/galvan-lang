use galvan_ast::{DeclModifier, TypeElement};
use galvan_hir::hir::*;
use itertools::Itertools;

use crate::codegen::{
    atomic_ordering, atomic_ref_storage_type, ref_storage_type, wrap_ref_storage_value,
};
use crate::context::Context;
use crate::macros::transpile;
use crate::sanitize::sanitize_name;
use crate::ErrorCollector;
use crate::Transpile;

impl Transpile for HirBlock {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        // Only blocks that produce a value keep their trailing expression
        // unterminated; in void blocks a trailing `;` makes sure temporaries
        // (e.g. mutex guards) are dropped before the block's locals
        let trailing = !self.is_void()
            && matches!(
                self.statements.last(),
                Some(HirStatement::Expression(_)) | Some(HirStatement::Return(_))
            );

        if trailing {
            let (init, last) = self.statements.split_at(self.statements.len() - 1);
            let last = last[0].transpile(ctx, errors);
            if init.is_empty() {
                return format!("{{\n{last}\n}}");
            }
            let statements = init
                .iter()
                .map(|statement| statement.transpile(ctx, errors))
                .join(";\n");
            format!("{{\n{statements};\n{last}\n}}")
        } else if self.statements.is_empty() {
            "{ }".to_string()
        } else {
            let statements = self
                .statements
                .iter()
                .map(|statement| statement.transpile(ctx, errors))
                .join(";\n");
            format!("{{\n{statements};\n}}")
        }
    }
}

impl Transpile for HirStatement {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self {
            HirStatement::Declaration(declaration) => declaration.transpile(ctx, errors),
            HirStatement::Assignment(assignment) => assignment.transpile(ctx, errors),
            HirStatement::Expression(expression) => expression.transpile(ctx, errors),
            HirStatement::Return(ret) => {
                let prefix = if ret.is_explicit { "return " } else { "" };
                transpile!(ctx, errors, "{prefix}{}", ret.expression)
            }
            HirStatement::Throw(throw) => {
                transpile!(ctx, errors, "return Err({})", throw.expression)
            }
            HirStatement::Break(_) => "break".to_string(),
            HirStatement::Continue(_) => "continue".to_string(),
        }
    }
}

impl Transpile for HirDeclaration {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let keyword = match self.modifier {
            DeclModifier::Let => "let",
            DeclModifier::Ref => "let mut",
            DeclModifier::Mut => "let mut",
            DeclModifier::Move => "let",
        };

        let identifier = sanitize_name(self.identifier.as_str());

        let ty = self.ty.transpile(ctx, errors);
        let ty = match self.modifier {
            DeclModifier::Let | DeclModifier::Mut | DeclModifier::Move => format!(": {ty}"),
            DeclModifier::Ref => format!(": {}", ref_storage_type(&self.ty, ty)),
        };

        match &self.value {
            Some(value) => {
                let rendered = value.transpile(ctx, errors);
                let value = if matches!(self.modifier, DeclModifier::Ref) {
                    wrap_ref_storage_value(rendered, value, &self.ty)
                } else {
                    rendered
                };
                format!("{keyword} {identifier}{ty} = {value}")
            }
            None => format!("{keyword} {identifier}{ty}"),
        }
    }
}

impl Transpile for HirAssignment {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        if self.deref_target && locks_atomic_target(&self.target) {
            return transpile_atomic_ref_assignment(self, ctx, errors);
        }

        let prefix = if self.deref_target { "*" } else { "" };

        // Assigning through an index on a dictionary or set becomes an insert
        if let HirExpressionKind::Index(index) = &self.target.kind {
            if matches!(
                index.base.ty,
                TypeElement::Dictionary(_)
                    | TypeElement::OrderedDictionary(_)
                    | TypeElement::Set(_)
            ) {
                match self.operator {
                    HirAssignmentOperator::Assign => {
                        return transpile!(
                            ctx,
                            errors,
                            "{}.insert({}, {})",
                            index.base,
                            index.index,
                            self.value
                        );
                    }
                    _ => {
                        let operator = combined_operator_symbol(&self.operator);
                        errors.error_with_span(
                            crate::TranspilerError::UnsupportedDictSetAssignment {
                                operation: operator.to_string(),
                                type_name: "dictionary or set".to_string(),
                            },
                            Some(self.span.into()),
                        );
                        return format!(
                            "/* Unsupported operation: {operator} on dictionary or set */"
                        );
                    }
                }
            }
        }

        match self.operator {
            HirAssignmentOperator::Assign => {
                transpile!(ctx, errors, "{prefix}{} = {}", self.target, self.value)
            }
            HirAssignmentOperator::AddAssign => {
                transpile!(ctx, errors, "{prefix}{} += {}", self.target, self.value)
            }
            HirAssignmentOperator::SubAssign => {
                transpile!(ctx, errors, "{prefix}{} -= {}", self.target, self.value)
            }
            HirAssignmentOperator::MulAssign => {
                transpile!(ctx, errors, "{prefix}{} *= {}", self.target, self.value)
            }
            HirAssignmentOperator::DivAssign => {
                transpile!(ctx, errors, "{prefix}{} /= {}", self.target, self.value)
            }
            HirAssignmentOperator::RemAssign => {
                transpile!(ctx, errors, "{prefix}{} %= {}", self.target, self.value)
            }
            // The target is rendered twice; bind a guard so `ref` targets
            // lock their mutex only once
            HirAssignmentOperator::PowAssign if locks_target(&self.target) => {
                transpile!(
                    ctx,
                    errors,
                    "{{ let mut __guard = {}; *__guard = __guard.pow({}); }}",
                    self.target,
                    self.value
                )
            }
            HirAssignmentOperator::PowAssign => {
                transpile!(
                    ctx,
                    errors,
                    "{prefix}{} = {}.pow({})",
                    self.target,
                    self.target,
                    self.value
                )
            }
            HirAssignmentOperator::ConcatAssign(kind) => {
                transpile_concat_assign(self, kind, ctx, errors, prefix)
            }
        }
    }
}

/// Whether the rendered target ends in a mutex lock (`ref` variables)
fn locks_target(target: &HirExpression) -> bool {
    target.adjustments.last() == Some(&Adjustment::LockRef)
}

fn locks_atomic_target(target: &HirExpression) -> bool {
    locks_target(target) && atomic_ref_storage_type(&target.ty).is_some()
}

fn transpile_atomic_ref_assignment(
    assignment: &HirAssignment,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    let target = transpile_without_final_lock(&assignment.target, ctx, errors);
    let value = assignment.value.transpile(ctx, errors);
    let ordering = atomic_ordering();

    match assignment.operator {
        HirAssignmentOperator::Assign => format!("{target}.store({value}, {ordering})"),
        HirAssignmentOperator::AddAssign => format!("{target}.fetch_add({value}, {ordering})"),
        HirAssignmentOperator::SubAssign => format!("{target}.fetch_sub({value}, {ordering})"),
        HirAssignmentOperator::MulAssign => {
            format!("{{ let mut __value = {target}.load({ordering}); __value *= {value}; {target}.store(__value, {ordering}); }}")
        }
        HirAssignmentOperator::DivAssign => {
            format!("{{ let mut __value = {target}.load({ordering}); __value /= {value}; {target}.store(__value, {ordering}); }}")
        }
        HirAssignmentOperator::RemAssign => {
            format!("{{ let mut __value = {target}.load({ordering}); __value %= {value}; {target}.store(__value, {ordering}); }}")
        }
        HirAssignmentOperator::PowAssign => {
            format!("{{ let mut __value = {target}.load({ordering}); __value = __value.pow({value}); {target}.store(__value, {ordering}); }}")
        }
        HirAssignmentOperator::ConcatAssign(kind) => {
            transpile_concat_assign(assignment, kind, ctx, errors, "")
        }
    }
}

fn transpile_without_final_lock(
    expression: &HirExpression,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    let mut base = expression.clone();
    if base.adjustments.last() == Some(&Adjustment::LockRef) {
        base.adjustments.pop();
    }
    base.transpile(ctx, errors)
}

/// `++=` appends an element or extends with a collection; the shape was
/// decided by the typechecker. Method-call shapes auto-(de)reference their
/// receiver, so they never need the deref prefix.
fn transpile_concat_assign(
    assignment: &HirAssignment,
    kind: ConcatKind,
    ctx: &Context,
    errors: &mut ErrorCollector,
    prefix: &str,
) -> String {
    match (&assignment.target.ty, kind) {
        (TypeElement::Array(_), ConcatKind::Element) => {
            transpile!(
                ctx,
                errors,
                "{}.push({})",
                assignment.target,
                assignment.value
            )
        }
        (TypeElement::Array(_), _) => {
            transpile!(
                ctx,
                errors,
                "{}.extend({})",
                assignment.target,
                assignment.value
            )
        }
        (TypeElement::Set(_), ConcatKind::Element) => {
            transpile!(
                ctx,
                errors,
                "{}.insert({})",
                assignment.target,
                assignment.value
            )
        }
        // The target is rendered twice; bind a guard so `ref` targets lock
        // their mutex only once
        (TypeElement::Set(_), _) if locks_target(&assignment.target) => {
            transpile!(
                ctx,
                errors,
                "{{ let mut __guard = {}; *__guard = __guard.union(&{}).cloned().collect::<::std::collections::HashSet<_>>(); }}",
                assignment.target,
                assignment.value
            )
        }
        (TypeElement::Set(_), _) => {
            transpile!(
                ctx,
                errors,
                "{prefix}{} = ({prefix}{}).union(&{}).cloned().collect::<::std::collections::HashSet<_>>()",
                assignment.target,
                assignment.target,
                assignment.value
            )
        }
        (TypeElement::Plain(basic), ConcatKind::Element) if basic.ident.as_str() == "String" => {
            transpile!(
                ctx,
                errors,
                "{}.push({})",
                assignment.target,
                assignment.value
            )
        }
        (TypeElement::Plain(basic), ConcatKind::Collection) if basic.ident.as_str() == "String" => {
            transpile!(
                ctx,
                errors,
                "{}.push_str(&{})",
                assignment.target,
                assignment.value
            )
        }
        (TypeElement::Plain(basic), ConcatKind::Stringify) if basic.ident.as_str() == "String" => {
            transpile!(
                ctx,
                errors,
                "{}.push_str(&{}.to_string())",
                assignment.target,
                assignment.value
            )
        }
        (_, ConcatKind::Element) => {
            transpile!(
                ctx,
                errors,
                "{}.push({})",
                assignment.target,
                assignment.value
            )
        }
        (_, _) => {
            transpile!(
                ctx,
                errors,
                "{}.extend({})",
                assignment.target,
                assignment.value
            )
        }
    }
}

fn combined_operator_symbol(operator: &HirAssignmentOperator) -> &'static str {
    match operator {
        HirAssignmentOperator::Assign => "=",
        HirAssignmentOperator::AddAssign => "+=",
        HirAssignmentOperator::SubAssign => "-=",
        HirAssignmentOperator::MulAssign => "*=",
        HirAssignmentOperator::DivAssign => "/=",
        HirAssignmentOperator::RemAssign => "%=",
        HirAssignmentOperator::PowAssign => "**=",
        HirAssignmentOperator::ConcatAssign(_) => "++=",
    }
}
