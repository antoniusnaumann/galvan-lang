use crate::context::Context;
use crate::error::ErrorCollector;
use crate::macros::impl_transpile_variants;
use crate::{Ast, RootItem, Transpile};
use galvan_resolver::Scope;

impl Transpile for Ast {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        self.toplevel.transpile(ctx, scope, errors)
    }
}

impl_transpile_variants!(RootItem; Type, Fn, Main, Test);
