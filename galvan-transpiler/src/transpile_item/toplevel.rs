use crate::context::Context;
use crate::macros::impl_transpile_variants;
use crate::{Ast, RootItem, Transpile};
use galvan_resolver::Scope;

impl Transpile for Ast {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        self.toplevel.transpile(ctx, scope)
    }
}

impl_transpile_variants!(RootItem; Type, Fn, Main, Test);
