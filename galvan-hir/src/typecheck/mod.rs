//! The Galvan typechecker: lowers the AST into the typed [HIR](crate::hir).
//!
//! Lowering performs name resolution, bidirectional type inference and
//! ownership analysis in a single pass. Expected types and ownership flow
//! *down* through [`Expected`] values, inferred types and ownership flow *up*
//! through the returned [`HirExpression`]s, and [`Checker::coerce`] reconciles
//! the two by attaching explicit [`Adjustment`](crate::hir::Adjustment)s.

mod coerce;
mod expr;
mod scope;

use galvan_ast::{
    Assignment, AssignmentOperator, Body, DeclModifier, Declaration, FnDecl, Ident, MainKind,
    Ownership, SegmentedAsts, Span, Statement, ToplevelItem, TypeElement,
};
use galvan_resolver::{LookupContext, LookupError};
use galvan_rustdoc::RustInterop;

use crate::builtins::{builtin_fns, builtins, predefined_from, CheckBuiltins};
use crate::error::ErrorCollector;
use crate::hir::*;
use crate::mapping::Mapping;

pub use scope::Variable;

pub use coerce::types_compatible;
pub(crate) use coerce::{concat_kind, Expected};
pub(crate) use scope::ScopeStack;

/// Typechecks a segmented AST and lowers it into a [`HirModule`].
///
/// Type and ownership errors do not abort lowering; they are reported through
/// the returned [`ErrorCollector`] so that callers can decide how to surface
/// them.
pub fn typecheck(asts: SegmentedAsts) -> Result<(HirModule, ErrorCollector), LookupError> {
    typecheck_with_interop(asts, &RustInterop::empty())
}

pub fn typecheck_with_interop(
    asts: SegmentedAsts,
    rust_interop: &RustInterop,
) -> Result<(HirModule, ErrorCollector), LookupError> {
    let mapping = builtins();
    let predefined = predefined_from(&mapping, builtin_fns());

    let (functions, tests, main, cmd_bodies, errors) = {
        let mut lookup = LookupContext::new().with(&predefined)?.with(&asts)?;
        for ty in rust_interop.imported_types() {
            lookup.types.entry(ty.name.clone()).or_insert(&ty.decl);
        }
        let mut checker = Checker::new(&lookup, &mapping, rust_interop);

        let functions = asts
            .functions
            .iter()
            .map(|func| checker.lower_function(func))
            .collect::<Vec<_>>();

        let tests = asts
            .tests
            .iter()
            .map(|test| HirTest {
                name: test.item.name.clone(),
                body: checker.lower_toplevel_body(&test.item.body),
                source: test.source.clone(),
            })
            .collect::<Vec<_>>();

        let main = asts.main.as_ref().map(|main| {
            checker.scopes.push();
            let kind = match &main.item.kind {
                MainKind::Command(signature) => {
                    for param in &signature.parameters.params {
                        checker.scopes.declare(Variable {
                            ident: param.identifier.clone(),
                            modifier: param.decl_modifier.unwrap_or(DeclModifier::Let),
                            ty: param.param_type.clone(),
                            ownership: Ownership::UniqueOwned,
                        });
                    }
                    HirMainKind::Command {
                        signature: signature.clone(),
                    }
                }
                MainKind::Function { argument } => {
                    if let Some(argument) = argument {
                        checker.scopes.declare(Variable {
                            ident: argument.identifier.clone(),
                            modifier: DeclModifier::Let,
                            ty: argument.param_type.clone(),
                            ownership: Ownership::UniqueOwned,
                        });
                    }
                    HirMainKind::Function {
                        argument: argument
                            .as_ref()
                            .map(|argument| argument.identifier.clone()),
                    }
                }
            };
            let body = checker.lower_toplevel_body(&main.item.body);
            checker.scopes.pop();

            HirMain {
                kind,
                body,
                source: main.source.clone(),
            }
        });

        let cmd_bodies = asts
            .cmds
            .iter()
            .map(|cmd| {
                checker.scopes.push();
                for param in &cmd.item.signature.parameters.params {
                    // CLI parameters are passed by value
                    checker.scopes.declare(Variable {
                        ident: param.identifier.clone(),
                        modifier: param.decl_modifier.unwrap_or(DeclModifier::Let),
                        ty: param.param_type.clone(),
                        ownership: Ownership::UniqueOwned,
                    });
                }
                let body = checker.lower_block(&cmd.item.body, &Expected::void());
                checker.scopes.pop();
                body
            })
            .collect::<Vec<_>>();

        (functions, tests, main, cmd_bodies, checker.errors)
    };

    let SegmentedAsts {
        uses, types, cmds, ..
    } = asts;
    let cmds = cmds
        .into_iter()
        .zip(cmd_bodies)
        .map(|(decl, body)| {
            let ToplevelItem { item, source } = decl;
            HirCmd {
                signature: item.signature,
                body,
                source,
                span: item.span,
            }
        })
        .collect();

    Ok((
        HirModule {
            uses,
            types,
            functions,
            tests,
            main,
            cmds,
        },
        errors,
    ))
}

pub(crate) struct Checker<'a> {
    pub(crate) lookup: &'a LookupContext<'a>,
    pub(crate) rust_interop: &'a RustInterop,
    pub(crate) mapping: &'a Mapping,
    pub(crate) scopes: ScopeStack,
    pub(crate) errors: ErrorCollector,
    /// Return type of the function currently being lowered
    pub(crate) fn_return: TypeElement,
    pub(crate) ref_self: bool,
}

impl<'a> Checker<'a> {
    fn new(
        lookup: &'a LookupContext<'a>,
        mapping: &'a Mapping,
        rust_interop: &'a RustInterop,
    ) -> Self {
        Self {
            lookup,
            rust_interop,
            mapping,
            scopes: ScopeStack::new(),
            errors: ErrorCollector::new(),
            fn_return: TypeElement::void(),
            ref_self: false,
        }
    }

    pub(crate) fn is_copy(&self, ty: &TypeElement) -> bool {
        self.mapping.is_copy(ty)
    }

    fn lower_function(&mut self, func: &ToplevelItem<FnDecl>) -> HirFunction {
        let signature = func.item.signature.clone();

        self.scopes.push();
        for param in &signature.parameters.params {
            let is_copy = self.is_copy(&param.param_type);
            let ownership = match param.decl_modifier {
                Some(DeclModifier::Let) | None => {
                    if is_copy {
                        Ownership::UniqueOwned
                    } else {
                        Ownership::Borrowed
                    }
                }
                Some(DeclModifier::Mut) => Ownership::MutBorrowed,
                Some(DeclModifier::Ref) => Ownership::Ref,
                Some(DeclModifier::Move) => Ownership::UniqueOwned,
            };
            self.scopes.declare(Variable {
                ident: param.identifier.clone(),
                modifier: param.decl_modifier.unwrap_or(DeclModifier::Let),
                ty: param.param_type.clone(),
                ownership,
            });
        }

        self.ref_self = signature
            .receiver()
            .is_some_and(|receiver| receiver.decl_modifier == Some(DeclModifier::Ref));
        self.fn_return = signature.return_type.clone();
        let expected = if signature.return_type.is_void() || signature.return_type.is_infer() {
            Expected::void()
        } else {
            Expected::owned(signature.return_type.clone())
        };
        let body = self.lower_block(&func.item.body, &expected);

        self.scopes.pop();
        self.fn_return = TypeElement::void();
        self.ref_self = false;

        HirFunction {
            signature,
            body,
            source: func.source.clone(),
            span: func.item.span,
        }
    }

    fn lower_toplevel_body(&mut self, body: &Body) -> HirBlock {
        self.fn_return = TypeElement::void();
        self.scopes.push();
        let block = self.lower_block(body, &Expected::void());
        self.scopes.pop();
        block
    }

    /// Lowers a body. When the context expects a value, the trailing
    /// expression is coerced to it; all other statements are lowered in
    /// statement position.
    pub(crate) fn lower_block(&mut self, body: &Body, expected: &Expected) -> HirBlock {
        self.scopes.push();

        let mut statements = Vec::with_capacity(body.statements.len());
        let last_index = body.statements.len().saturating_sub(1);
        let mut ty = TypeElement::void();

        for (i, statement) in body.statements.iter().enumerate() {
            let is_last = i == last_index;
            match statement {
                Statement::Expression(expression) if is_last && !expected.is_void() => {
                    let lowered = self.lower_expression(expression, expected);
                    ty = if matches!(lowered.ty, TypeElement::Never(_)) {
                        lowered.ty.clone()
                    } else if expected.is_free() {
                        lowered.ty.clone()
                    } else {
                        expected.ty.clone()
                    };
                    statements.push(HirStatement::Expression(lowered));
                }
                statement => {
                    let lowered = self.lower_statement(statement);
                    if is_last {
                        ty = match &lowered {
                            HirStatement::Return(_)
                            | HirStatement::Throw(_)
                            | HirStatement::Break(_)
                            | HirStatement::Continue(_) => {
                                TypeElement::Never(galvan_ast::NeverTypeItem {
                                    span: Span::default(),
                                })
                            }
                            _ => TypeElement::void(),
                        };
                    }
                    statements.push(lowered);
                }
            }
        }

        self.scopes.pop();

        HirBlock {
            statements,
            ty,
            span: body.span,
        }
    }

    pub(crate) fn lower_statement(&mut self, statement: &Statement) -> HirStatement {
        match statement {
            Statement::Declaration(declaration) => {
                HirStatement::Declaration(self.lower_declaration(declaration))
            }
            Statement::Assignment(assignment) => {
                HirStatement::Assignment(self.lower_assignment(assignment))
            }
            Statement::Expression(expression) => {
                HirStatement::Expression(self.lower_expression(expression, &Expected::void()))
            }
            Statement::Return(ret) => {
                let expected = if self.fn_return.is_void() || self.fn_return.is_infer() {
                    Expected::free()
                } else {
                    Expected::owned(self.fn_return.clone())
                };
                HirStatement::Return(HirReturn {
                    expression: self.lower_expression(&ret.expression, &expected),
                    is_explicit: ret.is_explicit,
                    span: ret.span,
                })
            }
            Statement::Throw(throw) => HirStatement::Throw(HirThrow {
                expression: self.lower_expression(&throw.expression, &Expected::free()),
                span: throw.span,
            }),
            Statement::Break(brk) => HirStatement::Break(brk.span),
            Statement::Continue(cont) => HirStatement::Continue(cont.span),
        }
    }

    fn lower_declaration(&mut self, declaration: &Declaration) -> HirDeclaration {
        let shares_ref = declaration.decl_modifier == DeclModifier::Ref
            && declaration.assignment_modifier == Some(DeclModifier::Ref);

        let (value, ty) = match (&declaration.type_annotation, &declaration.assignment) {
            (Some(annotation), Some(expression)) => {
                let expected =
                    self.declaration_expected(annotation, declaration.decl_modifier, shares_ref);
                let value = self.lower_modified_value(
                    expression,
                    declaration.assignment_modifier,
                    declaration.decl_modifier == DeclModifier::Ref,
                    "declaration initializers",
                );
                let value = self.coerce(value, &expected);
                (Some(value), annotation.clone())
            }
            (None, Some(expression)) => {
                // Infer the variable type from the initializer, then make
                // sure the initializer produces an owned value
                let value = self.lower_modified_value(
                    expression,
                    declaration.assignment_modifier,
                    declaration.decl_modifier == DeclModifier::Ref,
                    "declaration initializers",
                );
                let ty = value.ty.clone();
                let expected =
                    self.declaration_expected(&ty, declaration.decl_modifier, shares_ref);
                let value = self.coerce(value, &expected);
                (Some(value), ty)
            }
            (Some(annotation), None) => (None, annotation.clone()),
            (None, None) => {
                self.errors.warning(
                    format!(
                        "Variable '{}' needs a type annotation or an initializer",
                        declaration.identifier
                    ),
                    Some(declaration.span.into()),
                );
                (None, TypeElement::infer())
            }
        };

        let ownership = match declaration.decl_modifier {
            DeclModifier::Let | DeclModifier::Mut | DeclModifier::Move => {
                if self.is_copy(&ty) {
                    Ownership::UniqueOwned
                } else {
                    Ownership::SharedOwned
                }
            }
            DeclModifier::Ref => Ownership::Ref,
        };

        self.scopes.declare(Variable {
            ident: declaration.identifier.clone(),
            modifier: declaration.decl_modifier,
            ty: ty.clone(),
            ownership,
        });

        HirDeclaration {
            modifier: declaration.decl_modifier,
            identifier: declaration.identifier.clone(),
            ty,
            value,
            span: declaration.span,
        }
    }

    fn declaration_expected(
        &self,
        ty: &TypeElement,
        modifier: DeclModifier,
        shares_ref: bool,
    ) -> Expected {
        let ownership = match modifier {
            DeclModifier::Let | DeclModifier::Mut | DeclModifier::Move => {
                if self.is_copy(ty) {
                    Ownership::UniqueOwned
                } else {
                    Ownership::SharedOwned
                }
            }
            DeclModifier::Ref if shares_ref => Ownership::Ref,
            DeclModifier::Ref => Ownership::UniqueOwned,
        };
        Expected::with(ty.clone(), ownership)
    }

    fn lower_assignment(&mut self, assignment: &Assignment) -> HirAssignment {
        let mut target = self.lower_expression(&assignment.target, &Expected::free());
        let rebinds_ref = assignment.operator == AssignmentOperator::Assign
            && assignment.modifier == Some(DeclModifier::Ref);
        let assignment_accepts_ref = rebinds_ref && target.adjusted_ownership() == Ownership::Ref;

        // Assignments store through the place the target denotes; mutably
        // borrowed places are dereferenced and `ref` places go through the
        // mutex (`*x.lock().unwrap() = value`)
        let mut deref_target = false;
        if matches!(target.kind, HirExpressionKind::Variable(_)) {
            match target.ownership {
                Ownership::MutBorrowed => deref_target = true,
                Ownership::Ref => {
                    if !rebinds_ref {
                        target = target.adjusted(Adjustment::LockRef);
                        deref_target = true;
                    }
                }
                _ => {}
            }
        }

        let value = self.lower_modified_value(
            &assignment.expression,
            assignment.modifier,
            assignment_accepts_ref,
            "assignment right-hand sides",
        );
        let (operator, value) = self.lower_assignment_operator(assignment, &target, value);

        HirAssignment {
            target,
            deref_target,
            operator,
            value,
            span: assignment.span,
        }
    }

    /// Resolves the assignment operator and coerces the value to what the
    /// generated assignment consumes: an owned value of the place's type, or
    /// the shape determined by the `++=` classification.
    fn lower_assignment_operator(
        &mut self,
        assignment: &Assignment,
        target: &HirExpression,
        value: HirExpression,
    ) -> (HirAssignmentOperator, HirExpression) {
        let operator = match assignment.operator {
            AssignmentOperator::Assign => HirAssignmentOperator::Assign,
            AssignmentOperator::AddAssign => HirAssignmentOperator::AddAssign,
            AssignmentOperator::SubAssign => HirAssignmentOperator::SubAssign,
            AssignmentOperator::MulAssign => HirAssignmentOperator::MulAssign,
            AssignmentOperator::DivAssign => HirAssignmentOperator::DivAssign,
            AssignmentOperator::RemAssign => HirAssignmentOperator::RemAssign,
            AssignmentOperator::PowAssign => HirAssignmentOperator::PowAssign,
            AssignmentOperator::ConcatAssign => {
                let kind = concat_kind(&target.ty, &value.ty);
                let value = self.coerce_concat_value(&target.ty, kind, value, true);
                return (HirAssignmentOperator::ConcatAssign(kind), value);
            }
        };

        let value = self.coerce(value, &self.assignment_value_expected(target));
        (operator, value)
    }

    /// The expectation for the right-hand side of a non-concat assignment:
    /// an owned value of the place's type. The mutability of the place itself
    /// is handled through `deref_target`, never by adjusting the value.
    fn assignment_value_expected(&self, target: &HirExpression) -> Expected {
        // Assigning into an indexed dictionary or set inserts the value
        if let HirExpressionKind::Index(index) = &target.kind {
            match &index.base.ty {
                TypeElement::Dictionary(dict) => {
                    return Expected::owned(dict.value.clone());
                }
                TypeElement::OrderedDictionary(dict) => {
                    return Expected::owned(dict.value.clone());
                }
                TypeElement::Set(set) => {
                    return Expected::owned(set.elements.clone());
                }
                _ => {}
            }
        }

        Expected::owned(target.ty.clone())
    }

    /// Resolves a variable, reporting an error with a suggestion when the
    /// name is unknown
    pub(crate) fn variable(&mut self, ident: &Ident, span: Span) -> Option<Variable> {
        match self.scopes.get(ident) {
            Some(variable) => Some(variable.clone()),
            None => {
                let available = self.scopes.variable_names();
                self.errors.suggest_similar_identifier(
                    ident.as_str(),
                    &available,
                    Some(span.into()),
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests;
