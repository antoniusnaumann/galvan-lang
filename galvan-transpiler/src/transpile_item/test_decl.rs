use crate::context::Context;
use crate::macros::transpile;
use crate::Transpile;
use galvan_ast::TestDecl;
use galvan_resolver::Scope;
use std::borrow::Cow;

impl Transpile for &(Cow<'_, str>, &TestDecl) {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let (name, test_decl) = self;
        let name = name.as_ref();
        transpile!(
            ctx,
            scope,
            "#[test]\nfn {}() {{\n{}\n}}",
            name,
            test_decl.body
        )
    }
}
