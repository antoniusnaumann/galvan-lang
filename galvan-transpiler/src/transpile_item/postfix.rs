use galvan_ast::{AccessExpression, YeetExpression};
use galvan_resolver::Scope;

use crate::error::ErrorCollector;
use crate::{context::Context, macros::transpile, Transpile};

impl Transpile for AccessExpression {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // TODO: typecheck that base is a collection and the key types matches
        transpile!(ctx, scope, errors, "{}[{}]", self.base, self.index)
    }
}

impl Transpile for YeetExpression {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // TODO: check that type is error or optional 
        // TODO: check that we are inside a function that returns a compatible error
        transpile!(ctx, scope, errors, "{}?", self.inner)
    }
}
