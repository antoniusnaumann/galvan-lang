use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::Transpile;
use galvan_ast::{
    Block, Closure, ClosureArgument, DeclModifier, ElseExpression, LetKeyword, Ownership, Param,
};
use galvan_resolver::{Scope, Variable};

impl Transpile for Closure {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let mut closure_scope = Scope::child(scope);
        let scope = &mut closure_scope;

        let arguments = self.arguments.transpile(ctx, scope);
        let block = self.block.transpile(ctx, scope);
        transpile!(ctx, scope, "|{}| {}", arguments, block)
    }
}

impl_transpile!(Block, "{}", body);
// TODO: Allow a second variant that takes an error as an argument
impl_transpile!(ElseExpression, "({}).__or_else(|| {})", receiver, block);

impl Transpile for ClosureArgument {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        // TODO: Type inference
        scope.declare_variable(Variable {
            ident: self.ident.clone(),
            modifier: DeclModifier::Let(LetKeyword), // TODO: Closure arg modifiers self.modifier.clone(),
            ty: self.ty.clone(),
            ownership: Ownership::Borrowed,
        });

        if let Some(ty) = &self.ty {
            let param = Param {
                identifier: self.ident.clone(),
                decl_modifier: None,
                param_type: ty.clone(),
            };

            transpile!(ctx, scope, "{}", param)
        } else {
            // TODO: Handle refs and mut here as well
            self.ident.transpile(ctx, scope)
        }
    }
}
