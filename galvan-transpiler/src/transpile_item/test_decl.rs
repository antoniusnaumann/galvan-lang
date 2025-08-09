use crate::context::Context;
use crate::macros::transpile;
use crate::Transpile;
use galvan_ast::{Ownership, TestDecl, TypeElement};
use galvan_resolver::Scope;
use std::borrow::Cow;

impl Transpile for &(Cow<'_, str>, &TestDecl) {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut crate::ErrorCollector) -> String {
        let (name, test_decl) = self;
        let name = name.as_ref();
        let mut scope = Scope::child(scope).returns(TypeElement::void(), Ownership::default());
        let scope = &mut scope;
        transpile!(
            ctx,
            scope,
            errors,
            "#[test]\nfn {}() {{\n{};\n}}",
            name,
            test_decl.body
        )
    }
}
