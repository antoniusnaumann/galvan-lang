use crate::context::Context;
use crate::macros::transpile;
use crate::Transpile;
use galvan_ast::TestDecl;
use std::borrow::Cow;

impl Transpile for &(Cow<'_, str>, &TestDecl) {
    fn transpile(&self, ctx: &Context) -> String {
        let (name, test_decl) = self;
        let name = name.as_ref();
        transpile!(ctx, "#[test]\nfn {}() {{\n{}\n}}", name, test_decl.body)
    }
}
