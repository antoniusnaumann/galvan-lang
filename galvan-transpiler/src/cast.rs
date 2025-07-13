use galvan_ast::{Expression, ExpressionKind, TypeElement};
use galvan_resolver::Scope;

use crate::{
    builtins::{CheckBuiltins, IsSame},
    context::Context,
    transpile,
    type_inference::InferType,
    Transpile,
};

// TODO: return a result type here
pub fn cast(
    expression: &Expression,
    type_: &Option<TypeElement>,
    ctx: &Context<'_>,
    scope: &mut Scope<'_>,
) -> String {
    // println!(
    //     "cargo::warning=expected: {:#?}, a: {:#?}",
    //     type_,
    //     expression.infer_type(scope)
    // );
    if let Some(expected) = type_ {
        let Some(ref actual) = expression.infer_type(scope) else {
            return expression.transpile(ctx, scope);
        };
        match (expected, actual) {
            (expected, actual) if expected.is_same(actual) || actual.is_infer() => {
                expression.transpile(ctx, scope)
            }
            (_, TypeElement::Never(_)) => expression.transpile(ctx, scope),
            (TypeElement::Optional(some), actual) if some.inner.is_same(actual) => {
                transpile!(ctx, scope, "Some({})", expression)
            }
            (TypeElement::Result(res), actual) if res.success.is_same(actual) => {
                transpile!(ctx, scope, "Ok({})", expression)
            }
            (TypeElement::Result(res), actual)
                if res
                    .error
                    .as_ref()
                    .is_some_and(|inner| inner.is_same(actual)) =>
            {
                // TODO: This should not be autocast but instead require a "throw" keyword
                transpile!(ctx, scope, "Err({})", expression)
            }
            (TypeElement::Result(_), actual) => {
                println!(
                    "cargo::warning=wrapping non-matching type {:?} in ok",
                    actual
                );
                transpile!(ctx, scope, "Ok({})", expression)
            }
            (TypeElement::Optional(_), actual) => {
                println!(
                    "cargo::warning=wrapping non-matching type {:?} in some",
                    actual
                );
                transpile!(ctx, scope, "Some({})", expression)
            }
            (_, _) => {
                // Let Rust try to figure this out
                transpile!(ctx, scope, "{}.into()", expression)
            }
        }
    } else {
        expression.transpile(ctx, scope)
    }
}
