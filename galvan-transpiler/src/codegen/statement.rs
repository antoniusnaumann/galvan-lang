use galvan_ast::{AssignmentOperator, DeclModifier, TypeElement};
use galvan_hir::hir::*;
use itertools::Itertools;

use super::{rhs_is_array_collection, rhs_is_set_collection};
use crate::context::Context;
use crate::error::ErrorCollector;
use crate::macros::transpile;
use crate::sanitize::sanitize_name;
use crate::Transpile;

impl Transpile for HirBlock {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let trailing = matches!(
            self.statements.last(),
            Some(HirStatement::Expression(_)) | Some(HirStatement::Return(_))
        );

        if trailing {
            let (init, last) = self.statements.split_at(self.statements.len() - 1);
            let statements = init
                .iter()
                .map(|statement| statement.transpile(ctx, errors))
                .join(";\n");
            let last = last[0].transpile(ctx, errors);
            format!("{{\n{statements};\n{last}\n}}")
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
            DeclModifier::Let | DeclModifier::Ref => "let",
            DeclModifier::Mut => "let mut",
        };

        let identifier = sanitize_name(self.identifier.as_str());

        let ty = self.ty.transpile(ctx, errors);
        let ty = match self.modifier {
            DeclModifier::Let | DeclModifier::Mut => format!(": {ty}"),
            DeclModifier::Ref => format!(": std::sync::Arc<std::sync::Mutex<{ty}>>"),
        };

        match &self.value {
            Some(value) => {
                let value = value.transpile(ctx, errors);
                let value = if matches!(self.modifier, DeclModifier::Ref) {
                    format!("(&({value})).__to_ref()")
                } else {
                    value
                };
                format!("{keyword} {identifier}{ty} = {value}")
            }
            None => format!("{keyword} {identifier}{ty}"),
        }
    }
}

impl Transpile for HirAssignment {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
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
                    AssignmentOperator::Assign => {
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
                        errors.error(crate::TranspilerError::UnsupportedDictSetAssignment {
                            operation: operator.to_string(),
                            type_name: "dictionary or set".to_string(),
                        });
                        return format!(
                            "/* Unsupported operation: {operator} on dictionary or set */"
                        );
                    }
                }
            }
        }

        match self.operator {
            AssignmentOperator::Assign => {
                transpile!(ctx, errors, "{prefix}{} = {}", self.target, self.value)
            }
            AssignmentOperator::AddAssign => {
                transpile!(ctx, errors, "{prefix}{} += {}", self.target, self.value)
            }
            AssignmentOperator::SubAssign => {
                transpile!(ctx, errors, "{prefix}{} -= {}", self.target, self.value)
            }
            AssignmentOperator::MulAssign => {
                transpile!(ctx, errors, "{prefix}{} *= {}", self.target, self.value)
            }
            AssignmentOperator::DivAssign => {
                transpile!(ctx, errors, "{prefix}{} /= {}", self.target, self.value)
            }
            AssignmentOperator::RemAssign => {
                transpile!(ctx, errors, "{prefix}{} %= {}", self.target, self.value)
            }
            AssignmentOperator::PowAssign => {
                transpile!(
                    ctx,
                    errors,
                    "{prefix}{} = {}.pow({})",
                    self.target,
                    self.target,
                    self.value
                )
            }
            AssignmentOperator::ConcatAssign => {
                transpile_concat_assign(self, ctx, errors, prefix)
            }
        }
    }
}

/// `++=` appends an element or extends with a collection depending on the
/// stored operand types
fn transpile_concat_assign(
    assignment: &HirAssignment,
    ctx: &Context,
    errors: &mut ErrorCollector,
    prefix: &str,
) -> String {
    match &assignment.target.ty {
        TypeElement::Array(array) => {
            if rhs_is_array_collection(&array.elements, &assignment.value.ty) {
                transpile!(
                    ctx,
                    errors,
                    "{prefix}{}.extend({})",
                    assignment.target,
                    assignment.value
                )
            } else {
                transpile!(
                    ctx,
                    errors,
                    "{prefix}{}.push({})",
                    assignment.target,
                    assignment.value
                )
            }
        }
        TypeElement::Set(set) => {
            if rhs_is_set_collection(&set.elements, &assignment.value.ty) {
                transpile!(
                    ctx,
                    errors,
                    "{} = {prefix}{}.union(&{}).cloned().collect::<::std::collections::HashSet<_>>().to_owned()",
                    assignment.target,
                    assignment.target,
                    assignment.value
                )
            } else {
                transpile!(
                    ctx,
                    errors,
                    "{prefix}{}.insert({})",
                    assignment.target,
                    assignment.value
                )
            }
        }
        TypeElement::Plain(basic) if basic.ident.as_str() == "String" => {
            if let TypeElement::Plain(value_ty) = &assignment.value.ty {
                if value_ty.ident.as_str() == "Char" {
                    return transpile!(
                        ctx,
                        errors,
                        "{prefix}{}.push({})",
                        assignment.target,
                        assignment.value
                    );
                } else if value_ty.ident.as_str() == "String" {
                    return transpile!(
                        ctx,
                        errors,
                        "{prefix}{}.push_str(&{})",
                        assignment.target,
                        assignment.value
                    );
                }
            }
            transpile!(
                ctx,
                errors,
                "{prefix}{}.push_str(&{}.to_string())",
                assignment.target,
                assignment.value
            )
        }
        _ => {
            transpile!(
                ctx,
                errors,
                "{prefix}{}.extend({})",
                assignment.target,
                assignment.value
            )
        }
    }
}

fn combined_operator_symbol(operator: &AssignmentOperator) -> &'static str {
    match operator {
        AssignmentOperator::Assign => "=",
        AssignmentOperator::AddAssign => "+=",
        AssignmentOperator::SubAssign => "-=",
        AssignmentOperator::MulAssign => "*=",
        AssignmentOperator::DivAssign => "/=",
        AssignmentOperator::RemAssign => "%=",
        AssignmentOperator::PowAssign => "**=",
        AssignmentOperator::ConcatAssign => "++=",
    }
}
