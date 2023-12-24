use crate::macros::impl_transpile;
use crate::{LookupContext, MainDecl, Transpile};
use galvan_ast::TestDecl;

impl_transpile!(MainDecl, "fn main() {{ {} }}", body);

impl Transpile for TestDecl {
    fn transpile(&self, lookup: &LookupContext) -> String {
        let name: &str = self.name.as_ref().map_or("test", |name| name.as_str());
        // TODO: Collect all test functions into a single test module
        format!(
            "#[test]\nfn {}() {{ {} }}",
            name,
            self.body.transpile(lookup)
        )
    }
}
