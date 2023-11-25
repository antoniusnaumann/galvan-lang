use crate::{Transpile, Ast, RootItem, transpile};
impl Transpile for Ast {
    fn transpile(self) -> String {
        self.toplevel.transpile()
    }
}

impl Transpile for RootItem {
    fn transpile(self) -> String {
        match self {
            // RootItem::Fn(f) => todo!(),
            RootItem::Type(t) => transpile!("{}", t),
            RootItem::Main(m) => transpile!("{}", m),
            RootItem::Test(t) => todo!(),
            RootItem::CustomTask(t) => todo!(),
        }
    }
}