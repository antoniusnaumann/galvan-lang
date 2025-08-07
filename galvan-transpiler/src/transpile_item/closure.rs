use std::ops::Deref;

use crate::cast::{self, cast, unify};
use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::type_inference::InferType;
use crate::Transpile;
use galvan_ast::{
    AstNode, Block, Closure, ClosureParameter, DeclModifier, ElseExpression, Expression,
    ExpressionKind, FunctionCall, Ownership, Param, ResultTypeItem, Span, TypeElement,
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
        .map(|a| transpile_closure_argument(ctx, scope, a, deref_args, Ownership::default(), true))
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
                let (cond, if_, if_ty) = transpile_if(&call, ctx, scope, scope.return_type.clone());
                let else_ = self.block.transpile(ctx, scope);
                let (if_, else_) = unify(&if_, &else_, &if_ty, &self.block.infer_type(scope));
                format!("{cond} {{ {if_} }} else {{ {else_} }}")
            }
            // special handling for try-else
            ExpressionKind::FunctionCall(call) if call.identifier.as_str() == "try" => {
                transpile_try(&call, ctx, scope, TypeElement::default(), Some(self))
            }
            _ => {
                let mut else_scope = Scope::child(scope);
                let block = self.block.transpile(ctx, &mut else_scope);
                
                // Determine the inner type of the optional for __value
                let receiver_type = self.receiver.infer_type(scope);
                let inner_type = match &receiver_type {
                    TypeElement::Optional(opt) => opt.inner.clone(),
                    _ => receiver_type, // fallback for non-optional types
                };
                
                // Create a child scope for the Some arm to declare __value
                let mut some_scope = Scope::child(scope);
                
                // Declare __value variable in the Some arm scope
                some_scope.declare_variable(Variable {
                    ident: "__value".to_owned().into(),
                    modifier: DeclModifier::Let,
                    ty: inner_type,
                    ownership: Ownership::Borrowed, // pattern match creates a borrowed reference
                });
                
                let cast_result = cast(
                    &Expression {
                        kind: ExpressionKind::Ident("__value".to_owned().into()),
                        span: Span::default()
                    },
                    &scope.return_type.clone(),
                    scope.ownership,
                    ctx,
                    &mut some_scope
                );
                
                transpile!(
                    ctx,
                    scope,
                    "if let Some(__value) = {} {{ {} }} else {{ {block} }}",
                    self.receiver,
                    cast_result
                )
            }
        }
    }
}

fn transpile_try(
    func: &FunctionCall,
    ctx: &Context<'_>,
    scope: &mut Scope,
    ty: TypeElement,
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
    let cond_ownership = condition.expression.infer_owned(ctx, scope);
    let mut cond_scope = Scope::child(scope).returns(cond_type.clone(), cond_ownership);
    let condition = cast(
        &condition.expression,
        &cond_type,
        cond_ownership,
        ctx,
        &mut cond_scope,
    );
    
    // For borrowed conditions in match expressions, we need to explicitly add &
    let condition = if cond_ownership == Ownership::Borrowed && matches!(cond_type, TypeElement::Optional(_) | TypeElement::Result(_)) {
        format!("&{}", condition)
    } else {
        condition
    };
    // let condition = if let Some(ref cond_type) = cond_type {
    //     if ctx.mapping.is_copy(&cond_type) {
    //         condition.strip_prefix("&").unwrap()
    //     } else {
    //         &condition
    //     }
    // } else {
    //     &condition
    // };

    let mut body_scope = Scope::child(scope).returns(ty.clone(), scope.ownership);
    let mut else_scope = Scope::child(scope).returns(ty, scope.ownership);

    match cond_type {
        TypeElement::Optional(_) | TypeElement::Infer(_) => {
            // TODO: allow more arguments for automatic tuple unpacking
            assert_eq!(
                body.parameters.len(),
                1,
                "'try' should have exactly one binding"
            );
            let is_copy = match &cond_type {
                TypeElement::Optional(opt) => ctx.mapping.is_copy(&opt.inner),
                TypeElement::Infer(_) => {
                    // For Infer types in Optional context, we can't determine the exact inner type,
                    // but we can make a reasonable assumption: if this is a try on an optional value
                    // that will be pattern matched, the bound variable should typically be owned
                    // rather than borrowed, especially for primitive types.
                    true
                }
                _ => ctx.mapping.is_copy(&cond_type),
            };
            let binding = {
                let elements = body
                    .parameters
                    .iter()
                    .map(|arg| {
                        // TODO: for tuple unpacking, we need to check this for each part of the tuple individually
                        let determined_ownership = if is_copy {
                            Ownership::UniqueOwned
                        } else {
                            Ownership::Borrowed
                        };
                        transpile_closure_argument(
                            ctx,
                            &mut body_scope,
                            arg,
                            false,
                            determined_ownership,
                            true,
                        )
                    })
                    .join(", ");
                format!("({elements})")
            };

            let transpiled_body = body.block.transpile(ctx, &mut body_scope);
            let else_ = else_
                .map(|else_| else_.block.transpile(ctx, &mut else_scope))
                .unwrap_or("{}".into());
            format!("match {condition} {{ Some({binding}) => {transpiled_body}, None => {else_} }}",)
        }
        TypeElement::Result(res) => {
            let ResultTypeItem {
                success,
                error,
                span: _,
            } = *res;
            let success_ownership = if ctx.mapping.is_copy(&success) {
                Ownership::UniqueOwned
            } else {
                Ownership::Borrowed
            };
            let error_ownership = if error.is_some_and(|error| ctx.mapping.is_copy(&error)) {
                Ownership::UniqueOwned
            } else {
                Ownership::Borrowed
            };
            let ok_binding = transpile_closure_argument(
                ctx,
                &mut body_scope,
                &body.parameters[0],
                false,
                success_ownership,
                true,
            );
            let err_binding = else_
                .and_then(|else_| else_.parameters.get(0))
                .map(|p| {
                    transpile_closure_argument(
                        ctx,
                        &mut else_scope,
                        p,
                        false,
                        error_ownership,
                        true,
                    )
                })
                .unwrap_or("_".into());
            let transpiled_body = body.block.transpile(ctx, &mut body_scope);
            let else_ = else_
                .map(|else_| else_.block.transpile(ctx, &mut else_scope))
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
        transpile_closure_argument(ctx, scope, self, false, Ownership::Borrowed, false)
    }
}

pub(crate) fn transpile_closure_argument(
    ctx: &Context,
    scope: &mut Scope,
    arg: &ClosureParameter,
    deref: bool,
    ownership: Ownership,
    omit_type: bool,
) -> String {
    // TODO: Type inference
    scope.declare_variable(Variable {
        ident: arg.ident.clone(),
        modifier: DeclModifier::Let, // TODO: Closure arg modifiers self.modifier.clone(),
        ty: arg.ty.clone(),
        ownership,
    });

    let prefix = if deref { "&" } else { "" };
    if omit_type {
        transpile!(ctx, scope, "{prefix}{}", arg.ident)
    } else {
        let param = Param {
            identifier: arg.ident.clone(),
            decl_modifier: None,
            param_type: arg.ty.clone(),
            span: arg.ty.span(),
        };
        transpile!(ctx, scope, "{prefix}{}", param)
    }
}
