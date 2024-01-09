use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::Transpile;
use galvan_ast::{Block, Closure, ClosureArgument, ElseExpression};
use galvan_resolver::Scope;

impl_transpile!(Closure, "|{}| {}", arguments, block);
impl_transpile!(Block, "{}", body);
// TODO: Allow a second variant that takes an error as an argument
impl_transpile!(ElseExpression, "({}).__or_else(|| {})", receiver, block);

impl Transpile for ClosureArgument {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        if let Some(ty) = &self.ty {
            transpile!(ctx, scope, "{}: {}", self.ident, ty)
        } else {
            // TODO: Handle refs and mut here as well
            self.ident.transpile(ctx, scope)
        }
    }
}
