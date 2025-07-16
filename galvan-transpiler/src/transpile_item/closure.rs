use std::ops::Deref;

use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::type_inference::InferType;
use crate::Transpile;
use galvan_ast::{
    AstNode, Block, Closure, ClosureParameter, DeclModifier, ElseExpression, ExpressionKind,
    FunctionCall, OptionalTypeItem, Ownership, Param, TypeElement,
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
        .parameters
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
            // special handling for try-else
            ExpressionKind::FunctionCall(call) if call.identifier.as_str() == "try" => {
                transpile_try(&call, ctx, scope, None, Some(self))
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

fn transpile_try(
    func: &FunctionCall,
    ctx: &Context<'_>,
    scope: &mut Scope,
    ty: Option<TypeElement>,
    else_: Option<&ElseExpression>,
) -> String {
    debug_assert_eq!(func.identifier.as_str(), "try");
    // TODO: allow more arguments for automatic tuple unpacking
    assert_eq!(
        func.arguments.len(),
        2,
        "try should have two arguments: condition and body"
    );
    let condition = &func.arguments[0];
    // TODO: relax this to 'last argument' for automatic tuple unpacking
    let ExpressionKind::Closure(body) = &func.arguments[1].expression.kind else {
        todo!("TRANSPILER ERROR: last argument of try needs to be a body")
    };
    let cond_type = condition.expression.infer_type(scope);
    let condition = condition.transpile(ctx, scope);
    let condition = if let Some(ref cond_type) = cond_type {
        if ctx.mapping.is_copy_type(&cond_type) {
            condition.strip_prefix("&").unwrap()
        } else {
            &condition
        }
    } else {
        &condition
    };

    let mut body_scope = Scope::child(scope);
    body_scope.return_type = ty.clone();
    let mut else_scope = Scope::child(scope);
    else_scope.return_type = ty;

    match cond_type {
        Some(TypeElement::Optional(_)) | None => {
            // TODO: allow more arguments for automatic tuple unpacking
            assert_eq!(
                body.parameters.len(),
                1,
                "'try' should have exactly one binding"
            );
            let binding = body.parameters[0].transpile(ctx, &mut body_scope);
            let transpiled_body = body.block.transpile(ctx, &mut body_scope);
            let else_ = else_
                .map(|else_| else_.block.transpile(ctx, scope))
                .unwrap_or("{}".into());
            format!("match {condition} {{ Some({binding}) => {transpiled_body}, None => {else_} }}",)
        }
        Some(TypeElement::Result(_)) => {
            let ok_binding = body.parameters[0].transpile(ctx, &mut body_scope);
            let err_binding = else_
                .and_then(|else_| else_.parameters.get(0))
                .map(|p| p.transpile(ctx, &mut else_scope))
                .unwrap_or("_".into());
            let transpiled_body = body.block.transpile(ctx, &mut body_scope);
            let else_ = else_
                .map(|else_| else_.block.transpile(ctx, scope))
                .unwrap_or("{}".into());
            format!(
                "match {condition} {{ Ok({ok_binding}) => {transpiled_body}, Err({err_binding}) => {else_} }}"
            )
        }
        _ => todo!("TRANSPILER ERROR: can only call 'try' on optionals or results"),
    }
}

impl Transpile for ClosureParameter {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        transpile_closure_argument(ctx, scope, self, false)
    }
}

fn transpile_closure_argument(
    ctx: &Context,
    scope: &mut Scope,
    arg: &ClosureParameter,
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
