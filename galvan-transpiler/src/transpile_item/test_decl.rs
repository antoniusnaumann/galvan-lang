use crate::context::Context;
use crate::macros::transpile;
use crate::Transpile;
use galvan_ast::{TestDecl, TypeElement};
use galvan_resolver::Scope;
use std::borrow::Cow;

impl Transpile for &(Cow<'_, str>, &TestDecl) {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let (name, test_decl) = self;
        let name = name.as_ref();
        let mut scope = Scope::child(scope).returns(TypeElement::void());
        let scope = &mut scope;
        transpile!(
            ctx,
            scope,
            "#[test]\nfn {}() {{\n{};\n}}",
            name,
            test_decl.body
        )
    }
}
