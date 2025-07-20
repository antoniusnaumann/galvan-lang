use std::borrow::Cow;

use galvan_ast::{BasicTypeItem, Expression, Ownership, TypeElement};
use galvan_resolver::Scope;

use crate::{
    builtins::{CheckBuiltins, IsSame},
    context::Context,
    transpile,
    type_inference::InferType,
    Transpile,
};

pub fn transpile_unified(
    a: &Expression,
    b: &Expression,
    a_scope: &mut Scope,
    b_scope: &mut Scope,
    ctx: &Context<'_>,
) -> (String, String) {
    let a_ty = a.infer_type(a_scope);
    let b_ty = b.infer_type(b_scope);

    let a = a.transpile(ctx, a_scope);
    let b = b.transpile(ctx, b_scope);

    let (a, b) = unify(&a, &b, &a_ty, &b_ty);
    (a.into_owned(), b.into_owned())
    // (
    //     format!("/*{}*/{}", crate::extension_name(&a_ty), a),
    //     format!("/*{}*/{}", crate::extension_name(&b_ty), b),
    // )
}

/// Unifies two already compiled expressions
pub fn unify<'a, 'b>(
    a: &'a str,
    b: &'b str,
    a_ty: &TypeElement,
    b_ty: &TypeElement,
) -> (Cow<'a, str>, Cow<'b, str>) {
    match (a_ty, b_ty) {
        (_, TypeElement::Never(_) | TypeElement::Infer(_))
        | (TypeElement::Never(_) | TypeElement::Infer(_), _) => (a.into(), b.into()),
        (expected, actual) if expected.is_same(actual) => (a.into(), b.into()),
        (a_ty, TypeElement::Optional(opt))
            // TODO: instead, try to unify the inner types
            if opt.inner.is_same(a_ty)
                || a_ty.is_infer()
                || a_ty.is_number()
                || opt.inner.is_infer()
                || opt.inner.is_number() =>
        {
            (format!("Some({a})").into(), b.into())
        }
        (TypeElement::Optional(opt), b_ty)
            if opt.inner.is_same(b_ty)
                || b_ty.is_infer()
                || b_ty.is_number()
                || opt.inner.is_infer()
                || opt.inner.is_number() =>
        {
            (a.into(), format!("Some({b})").into())
        }
        (a_ty, TypeElement::Result(res))
            if res.success.is_same(a_ty)
                || a_ty.is_infer()
                || a_ty.is_number()
                || res.success.is_infer()
                || res.success.is_number() =>
        {
            (format!("Ok({a})").into(), b.into())
        }
        (TypeElement::Result(res), b_ty)
            if res.success.is_same(b_ty)
                || b_ty.is_infer()
                || b_ty.is_number()
                || res.success.is_infer()
                || res.success.is_number() =>
        {
            (a.into(), format!("Ok({b})").into())
        }
        _ => (a.into(), b.into()),
    }
}

// TODO: return a result type here
pub fn cast(
    expression: &Expression,
    expected: &TypeElement,
    ctx: &Context<'_>,
    scope: &mut Scope<'_>,
) -> String {
    let ref actual = expression.infer_type(scope);

    // println!(
    //     "cargo::warning=Casting from {:#?} to {:#?}",
    //     crate::extension_name(actual),
    //     crate::extension_name(expected)
    // );

    match (expected, actual) {
        (expected, actual) if expected.is_same(actual) => expression.transpile(ctx, scope),
        (_, TypeElement::Never(_) | TypeElement::Infer(_) | TypeElement::Void(_)) => {
            expression.transpile(ctx, scope)
        }
        (TypeElement::Void(_) | TypeElement::Infer(_), _) => expression.transpile(ctx, scope),
        (TypeElement::Optional(some), actual) if some.inner.is_same(actual) => {
            let postfix = match expression.infer_owned(ctx, scope) {
                // TODO: we want to distinguish between Owned and SharedOwned, the latter needs to be cloned
                Ownership::Borrowed | Ownership::MutBorrowed => ".to_owned()",
                _ => ".to_owned()",
            };
            transpile!(ctx, scope, "Some({}{postfix})", expression)
        }
        (TypeElement::Result(res), actual) if res.success.is_same(actual) => {
            let postfix = match expression.infer_owned(ctx, scope) {
                Ownership::Borrowed | Ownership::MutBorrowed => ".to_owned()",
                _ => ".to_owned()",
            };
            transpile!(ctx, scope, "Ok({}{postfix})", expression)
        }
        (TypeElement::Result(res), actual)
            if res
                .error
                .as_ref()
                .is_some_and(|inner| inner.is_same(actual)) =>
        {
            // TODO: This should not be autocast but instead require a "throw" keyword
            transpile!(ctx, scope, "Err({})", expression)
        }
        (TypeElement::Result(_), actual) => {
            println!(
                "cargo::warning=wrapping non-matching type {:?} in ok",
                actual
            );
            let postfix = match expression.infer_owned(ctx, scope) {
                Ownership::Borrowed | Ownership::MutBorrowed => ".to_owned()",
                _ => ".to_owned()",
            };
            transpile!(ctx, scope, "/*non-matching*/Ok({}{postfix})", expression)
        }
        (TypeElement::Optional(_), TypeElement::Optional(_)) => {
            println!(
                "cargo::warning=wrapping non-matching type {:?} in some",
                actual
            );
            transpile!(ctx, scope, "/*non-matching*/{}", expression)
        }
        (TypeElement::Optional(_), actual) => {
            println!(
                "cargo::warning=wrapping non-matching type {:?} in some",
                actual
            );
            let postfix = match expression.infer_owned(ctx, scope) {
                Ownership::Borrowed | Ownership::MutBorrowed => ".to_owned()",
                _ => ".to_owned()",
            };
            transpile!(ctx, scope, "/*non-matching*/Some({}{postfix})", expression)
        }
        // TODO: only allow this for expected number types
        (_, TypeElement::Plain(BasicTypeItem { ident, span: _ }))
            if ident.as_str() == "__Number" =>
        {
            expression.transpile(ctx, scope)
        }
        (_, _) => {
            // Let Rust try to figure this out
            transpile!(ctx, scope, "{}.into()", expression)
        }
    }
}
