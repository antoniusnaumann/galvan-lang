use crate::context::Context;
use crate::macros::impl_transpile_variants;
use crate::{Ast, RootItem, Transpile};

impl Transpile for Ast {
    fn transpile(&self, ctx: &Context) -> String {
        self.toplevel.transpile(ctx)
    }
}

impl_transpile_variants!(RootItem; Type, Fn, Main, Test);
