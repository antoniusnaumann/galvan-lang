use galvan_ast::{AstNode, Expression, TypeElement};
use galvan_resolver::Scope;

use crate::{builtins::CheckBuiltins, type_inference::InferType};

// TODO: return a result type here
pub fn cast<'a>(
    expression: &'a Expression,
    type_: &Option<TypeElement>,
    scope: &Scope<'_>,
) -> Option<Expression> {
    if let Some(expected) = type_ {
        let Some(ref actual) = expression.infer_type(scope) else {
            return None;
        };
        match (expected, actual) {
            (expected, actual) if expected == actual => None,
            (_, actual) if actual.is_infer() => None,
            (expected, actual) => panic!(
                "Incompatible types! Expected: {}, got {}",
                expected.print(0),
                actual.print(0)
            ),
        }
    } else {
        None
    }
}
