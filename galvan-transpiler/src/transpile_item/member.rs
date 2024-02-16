use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::Transpile;
use galvan_ast::{ConstructorCall, ConstructorCallArg, MemberChain};
use galvan_resolver::Scope;
use itertools::Itertools;

impl Transpile for MemberChain {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        self.elements
            .iter()
            .map(|r| transpile!(ctx, scope, "{}", r))
            .collect_vec()
            .join(".")
    }
}

impl_transpile!(ConstructorCall, "{} {{ {} }}", identifier, arguments,);
impl_transpile!(ConstructorCallArg, "{}: {}", ident, expression);
