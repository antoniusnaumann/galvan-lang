use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::Transpile;
use galvan_ast::{
    ConstructorCall, ConstructorCallArg, MemberChainBase, MemberFieldAccess, MemberFunctionCall,
};
use galvan_resolver::Scope;
use itertools::Itertools;

impl Transpile for MemberFunctionCall {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self { base, call } = self;
        transpile!(ctx, scope, "{}.{}", base, call)
    }
}

impl Transpile for MemberFieldAccess {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self { base, field } = self;
        transpile!(ctx, scope, "{}.{}", base, field)
    }
}

impl Transpile for MemberChainBase {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        self.base
            .iter()
            .map(|r| transpile!(ctx, scope, "{}", r))
            .collect_vec()
            .join(".")
    }
}

impl_transpile!(ConstructorCall, "{} {{ {} }}", identifier, arguments,);
impl_transpile!(ConstructorCallArg, "{}: {}", ident, expression);
