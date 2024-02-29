use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::Transpile;
use galvan_ast::{
    Block, Closure, ClosureArgument, DeclModifier, ElseExpression, Ownership, Param,
};
use galvan_resolver::{Scope, Variable};
use itertools::Itertools;

impl Transpile for Closure {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        transpile_closure(ctx, scope, self, false)
    }
}

pub(crate) fn transpile_closure(
    ctx: &Context,
    scope: &mut Scope,
    closure: &Closure,
    deref_args: bool,
) -> String {
    let mut closure_scope = Scope::child(scope);
    let scope = &mut closure_scope;

    let arguments = closure
        .arguments
        .iter()
        .map(|a| transpile_closure_argument(ctx, scope, a, deref_args))
        .join(", ");
    let block = closure.block.transpile(ctx, scope);
    transpile!(ctx, scope, "|{}| {}", arguments, block)
}

impl_transpile!(Block, "{}", body);
// TODO: Allow a second variant that takes an error as an argument
impl_transpile!(ElseExpression, "({}).__or_else(|| {})", receiver, block);

impl Transpile for ClosureArgument {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        transpile_closure_argument(ctx, scope, self, false)
    }
}

fn transpile_closure_argument(
    ctx: &Context,
    scope: &mut Scope,
    arg: &ClosureArgument,
    deref: bool,
) -> String {
    // TODO: Type inference
    scope.declare_variable(Variable {
        ident: arg.ident.clone(),
        modifier: DeclModifier::Let, // TODO: Closure arg modifiers self.modifier.clone(),
        ty: arg.ty.clone(),
        ownership: Ownership::Borrowed,
    });

    let prefix = if deref { "&" } else { "" };
    if let Some(ty) = &arg.ty {
        let param = Param {
            identifier: arg.ident.clone(),
            decl_modifier: None,
            param_type: ty.clone(),
        };
        transpile!(ctx, scope, "{prefix}{}", param)
    } else {
        // TODO: Handle refs and mut here as well
        transpile!(ctx, scope, "{prefix}{}", arg.ident)
    }
}
