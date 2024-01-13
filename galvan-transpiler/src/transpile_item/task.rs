use crate::context::Context;
use crate::macros::impl_transpile;
use crate::{MainDecl, Transpile};
use galvan_ast::TestDecl;
use galvan_resolver::Scope;

impl_transpile!(MainDecl, "fn main() {}", body);

impl Transpile for TestDecl {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let name: &str = self.name.as_ref().map_or("test", |name| name.as_str());
        // TODO: Collect all test functions into a single test module
        format!(
            "#[test]\nfn {}() {{ {} }}",
            name,
            self.body.transpile(ctx, scope)
        )
    }
}
