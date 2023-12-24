use crate::macros::impl_transpile_variants;
use crate::{Ast, LookupContext, RootItem, Transpile};
impl Transpile for Ast {
    fn transpile(&self, lookup: &LookupContext) -> String {
        self.toplevel.transpile(lookup)
    }
}

impl_transpile_variants!(RootItem; Type, Main, Test);
