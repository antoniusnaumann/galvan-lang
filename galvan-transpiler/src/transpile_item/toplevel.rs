use crate::{Transpile, Ast, RootItem, impl_transpile_variants};
impl Transpile for Ast {
    fn transpile(self) -> String {
        self.toplevel.transpile()
    }
}

impl_transpile_variants!(RootItem; Type, Main, Test);