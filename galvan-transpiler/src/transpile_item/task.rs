use galvan_ast::{TestDecl};
use crate::{Transpile, MainDecl, impl_transpile};

impl_transpile!(MainDecl, "fn main() {{ {} }}", body);

impl Transpile for TestDecl {
    fn transpile(self) -> String {
        let name: String= self.name.map_or("test".into(), |name| name.into());
        // TODO: Collect all test functions into a single test module
        format!("#[test]\nfn {}() {{ {} }}", name, self.body.transpile())
    }
}