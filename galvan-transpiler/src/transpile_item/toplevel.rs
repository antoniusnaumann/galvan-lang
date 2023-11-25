use crate::{Transpile, Ast, RootItem, transpile, impl_transpile_match};
impl Transpile for Ast {
    fn transpile(self) -> String {
        self.toplevel.transpile()
    }
}

impl_transpile_match! { RootItem,
    Type(t) => ("{}", t),
    Main(m) => ("{}", m),
    Test(t) => ("{}", t),
    // CustomTask(t) => ("{}", t),
}