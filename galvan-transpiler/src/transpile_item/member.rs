use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::type_inference::InferType;
use crate::Transpile;
use galvan_ast::{ConstructorCall, ConstructorCallArg, InfixOperation, MemberOperator, Ownership};
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

impl crate::Transpile for ConstructorCallArg {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let postfix = match self.expression.infer_owned(ctx, scope) {
            Ownership::SharedOwned => ".clone()",
            Ownership::UniqueOwned => "",
            Ownership::Borrowed | Ownership::MutBorrowed | Ownership::Ref => ".to_owned()",
        };
        transpile!(ctx, scope, "{}: {}{postfix}", self.ident, self.expression)
    }
}
