use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::Transpile;
use galvan_ast::{ConstructorCall, ConstructorCallArg, MemberFieldAccess, MemberFunctionCall, SingleExpression};
use galvan_resolver::Scope;
use itertools::Itertools;

impl Transpile for MemberFunctionCall {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self { base, call } = self;
        let base = transpile_base(&base, ctx, scope);

        transpile!(ctx, scope, "{}.{}", base, call)
    }
}

impl Transpile for MemberFieldAccess {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self { base, field } = self;
        let base = transpile_base(&base, ctx, scope);

        transpile!(ctx, scope, "{}.{}", base, field)
    }
}

fn transpile_base(base: &[SingleExpression], ctx: &Context, scope: &mut Scope) -> String {
    base
        .iter()
        .map(|r| transpile!(ctx, scope, "{}", r))
        .collect_vec()
        .join(".")
}

impl_transpile!(ConstructorCall, "{} {{ {} }}", identifier, arguments,);
impl_transpile!(ConstructorCallArg, "{}: {}", ident, expression);
