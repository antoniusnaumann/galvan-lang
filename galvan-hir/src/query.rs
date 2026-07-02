//! Position-based queries over the typed HIR.
//!
//! The typechecker stores the inferred type, ownership and source span of
//! every expression in the HIR (see [`HirExpression`]). These queries map a
//! source position back to that information, e.g. to answer "what is the
//! type of the expression under the cursor" without re-running inference.

use std::path::Path;

use galvan_ast::Span;

use crate::hir::*;

/// The innermost expression covering byte `offset` in `file`.
///
/// The returned expression carries its inferred type ([`HirExpression::ty`])
/// and ownership, so this is the entry point for receiver-type queries
/// (member completion, hover over arbitrary expressions).
pub fn expression_at<'m>(
    module: &'m HirModule,
    file: &Path,
    offset: usize,
) -> Option<&'m HirExpression> {
    let mut best: Option<&HirExpression> = None;
    visit_expressions(module, file, &mut |expression| {
        if !contains(expression.span, offset) {
            return;
        }
        let replace = match best {
            Some(current) => width(expression.span) <= width(current.span),
            None => true,
        };
        if replace {
            best = Some(expression);
        }
    });
    best
}

fn contains(span: Span, offset: usize) -> bool {
    span.range.0 <= offset && offset < span.range.1
}

fn width(span: Span) -> usize {
    span.range.1.saturating_sub(span.range.0)
}

/// Call `visit` on every expression (in evaluation order, outermost first)
/// of every body that belongs to `file`.
pub fn visit_expressions<'m>(
    module: &'m HirModule,
    file: &Path,
    visit: &mut impl FnMut(&'m HirExpression),
) {
    for function in &module.functions {
        if function.source.origin() == Some(file) {
            visit_block(&function.body, visit);
        }
    }
    for test in &module.tests {
        if test.source.origin() == Some(file) {
            visit_block(&test.body, visit);
        }
    }
    if let Some(main) = &module.main {
        if main.source.origin() == Some(file) {
            visit_block(&main.body, visit);
        }
    }
    for cmd in &module.cmds {
        if cmd.source.origin() == Some(file) {
            visit_block(&cmd.body, visit);
        }
    }
}

fn visit_block<'m>(block: &'m HirBlock, visit: &mut impl FnMut(&'m HirExpression)) {
    for statement in &block.statements {
        visit_statement(statement, visit);
    }
}

fn visit_statement<'m>(statement: &'m HirStatement, visit: &mut impl FnMut(&'m HirExpression)) {
    match statement {
        HirStatement::Declaration(declaration) => {
            if let Some(value) = &declaration.value {
                visit_expression(value, visit);
            }
        }
        HirStatement::Assignment(assignment) => {
            visit_expression(&assignment.target, visit);
            visit_expression(&assignment.value, visit);
        }
        HirStatement::Expression(expression) => visit_expression(expression, visit),
        HirStatement::Return(ret) => visit_expression(&ret.expression, visit),
        HirStatement::Throw(throw) => visit_expression(&throw.expression, visit),
        HirStatement::Break(_) | HirStatement::Continue(_) => {}
    }
}

fn visit_expression<'m>(expression: &'m HirExpression, visit: &mut impl FnMut(&'m HirExpression)) {
    visit(expression);
    match &expression.kind {
        HirExpressionKind::If(if_expression) => {
            visit_expression(&if_expression.condition, visit);
            visit_block(&if_expression.then_block, visit);
            if let Some(else_block) = &if_expression.else_block {
                visit_block(else_block, visit);
            }
        }
        HirExpressionKind::ElseUnwrap(unwrap) => {
            visit_expression(&unwrap.receiver, visit);
            visit_block(&unwrap.else_block, visit);
        }
        HirExpressionKind::Try(try_expression) => {
            visit_expression(&try_expression.condition, visit);
            visit_block(&try_expression.body, visit);
            if let Some(else_block) = &try_expression.else_block {
                visit_block(else_block, visit);
            }
        }
        HirExpressionKind::For(for_expression) => {
            visit_expression(&for_expression.iterable, visit);
            visit_block(&for_expression.body, visit);
        }
        HirExpressionKind::Match(match_expression) => {
            visit_expression(&match_expression.scrutinee, visit);
            for arm in &match_expression.arms {
                visit_block(&arm.body, visit);
            }
        }
        HirExpressionKind::Assert(assert) => match assert.as_ref() {
            HirAssert::Eq(lhs, rhs, rest) | HirAssert::Ne(lhs, rhs, rest) => {
                visit_expression(lhs, visit);
                visit_expression(rhs, visit);
                for expression in rest {
                    visit_expression(expression, visit);
                }
            }
            HirAssert::Truthy(args) => {
                for expression in args {
                    visit_expression(expression, visit);
                }
            }
        },
        HirExpressionKind::Print(print) => {
            for arg in &print.args {
                visit_expression(arg, visit);
            }
        }
        HirExpressionKind::FunctionCall(call) => {
            for arg in &call.args {
                visit_expression(arg, visit);
            }
        }
        HirExpressionKind::MethodCall(call) => {
            visit_expression(&call.receiver, visit);
            for arg in &call.args {
                visit_expression(arg, visit);
            }
        }
        HirExpressionKind::FieldAccess(access) => visit_expression(&access.receiver, visit),
        HirExpressionKind::SafeAccess(access) => {
            visit_expression(&access.receiver, visit);
            if let SafeAccessKind::Call(_, _, _, args) = &access.access {
                for arg in args {
                    visit_expression(arg, visit);
                }
            }
        }
        HirExpressionKind::ConstructorCall(constructor) => {
            for arg in &constructor.args {
                visit_expression(&arg.value, visit);
            }
        }
        HirExpressionKind::EnumConstructor(constructor) => {
            for arg in &constructor.args {
                visit_expression(&arg.value, visit);
            }
        }
        HirExpressionKind::Literal(HirLiteral::String(string)) => {
            for interpolation in &string.interpolations {
                visit_expression(interpolation, visit);
            }
        }
        HirExpressionKind::Collection(collection) => match collection {
            HirCollection::Array(elements) | HirCollection::Set(elements) => {
                for element in elements {
                    visit_expression(element, visit);
                }
            }
            HirCollection::Dict(elements) | HirCollection::OrderedDict(elements) => {
                for element in elements {
                    visit_expression(&element.key, visit);
                    visit_expression(&element.value, visit);
                }
            }
        },
        HirExpressionKind::Closure(closure) => visit_block(&closure.body, visit),
        HirExpressionKind::Logical(binary) => visit_binary(binary, visit),
        HirExpressionKind::Arithmetic(binary) => visit_binary(binary, visit),
        HirExpressionKind::Bitwise(binary) => visit_binary(binary, visit),
        HirExpressionKind::Comparison(binary) => visit_binary(binary, visit),
        HirExpressionKind::CollectionOp(binary) => visit_binary(binary, visit),
        HirExpressionKind::Range(binary) => visit_binary(binary, visit),
        HirExpressionKind::Index(index) => {
            visit_expression(&index.base, visit);
            visit_expression(&index.index, visit);
        }
        HirExpressionKind::Yeet(inner) | HirExpressionKind::Group(inner) => {
            visit_expression(inner, visit);
        }
        HirExpressionKind::EnumAccess(_)
        | HirExpressionKind::RustConstant(_)
        | HirExpressionKind::Literal(_)
        | HirExpressionKind::Variable(_)
        | HirExpressionKind::Error(_) => {}
    }
}

fn visit_binary<'m, Op>(binary: &'m HirBinary<Op>, visit: &mut impl FnMut(&'m HirExpression)) {
    visit_expression(&binary.lhs, visit);
    visit_expression(&binary.rhs, visit);
}
