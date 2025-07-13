use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::Transpile;
use galvan_ast::{ConstructorCall, ConstructorCallArg, InfixOperation, MemberOperator};
use galvan_resolver::Scope;

impl Transpile for InfixOperation<MemberOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self { lhs, operator, rhs } = self;
        match operator {
            MemberOperator::Dot => transpile!(ctx, scope, "{}.{}", lhs, rhs),
            MemberOperator::SafeCall => {
                transpile!(ctx, scope, "{}.map(|__elem__| {{ __elem__.{} }})", lhs, rhs)
            }
        }
    }
}

impl_transpile!(ConstructorCall, "{} {{ {} }}", identifier, arguments,);
impl_transpile!(ConstructorCallArg, "{}: {}", ident, expression);
