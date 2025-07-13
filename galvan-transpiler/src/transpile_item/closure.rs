use std::ops::Deref;

use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::Transpile;
use galvan_ast::{
    AstNode, Block, Closure, ClosureArgument, DeclModifier, ElseExpression, ExpressionKind,
    Ownership, Param,
};
use galvan_resolver::{Scope, Variable};
use itertools::Itertools;

use super::function_call::transpile_if;

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

impl Transpile for ElseExpression {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        match &self.receiver.deref().kind {
            // special handling for if-else as opposed to using else on an optional value
            ExpressionKind::FunctionCall(call) if call.identifier.as_str() == "if" => {
                // TODO: we should attach the expected type to expressions somehow and honor that here
                let if_ = transpile_if(&call, ctx, scope, None);
                transpile!(ctx, scope, "{if_} else {{ {} }}", self.block)
            }
            _ => transpile!(
                ctx,
                scope,
                "({}).__or_else(|| {})",
                self.receiver,
                self.block
            ),
        }
    }
}

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
            span: ty.span(),
        };
        transpile!(ctx, scope, "{prefix}{}", param)
    } else {
        // TODO: Handle refs and mut here as well
        transpile!(ctx, scope, "{prefix}{}", arg.ident)
    }
}
