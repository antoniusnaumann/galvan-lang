use std::borrow::Cow;

use galvan_ast::{BasicTypeItem, Expression, Ownership, TypeElement};
use galvan_resolver::Scope;

use crate::{
    builtins::{CheckBuiltins, IsSame},
    context::Context,
    error::ErrorCollector,
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
    errors: &mut ErrorCollector,
) -> (String, String) {
    let a_ty = a.infer_type(a_scope, errors);
    let b_ty = b.infer_type(b_scope, errors);

    let a = a.transpile(ctx, a_scope, errors);
    let b = b.transpile(ctx, b_scope, errors);

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
        // Handle Optional and Result wrapping for __Number types
        (a_ty, TypeElement::Optional(opt))
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
        // Handle __Number comparisons - when one side is __Number, let Rust infer the type
        // This comes AFTER Optional/Result handling so wrapping takes priority
        (_, b_ty) if b_ty.is_number() => (a.into(), b.into()),
        (a_ty, _) if a_ty.is_number() => (a.into(), b.into()),
        _ => (a.into(), b.into()),
    }
}

// TODO: return a result type here
pub fn cast(
    expression: &Expression,
    expected: &TypeElement,
    ownership: Ownership,
    ctx: &Context<'_>,
    scope: &mut Scope<'_>,
    errors: &mut ErrorCollector,
) -> String {
    cast_with_errors(expression, expected, ownership, ctx, scope, errors)
}

pub fn cast_with_errors(
    expression: &Expression,
    expected: &TypeElement,
    ownership: Ownership,
    ctx: &Context<'_>,
    scope: &mut Scope<'_>,
    errors: &mut ErrorCollector,
) -> String {
    let ref actual = expression.infer_type(scope, errors);

    // println!(
    //     "cargo::warning=Casting from {:#?} to {:#?}",
    //     crate::extension_name(actual),
    //     crate::extension_name(expected)
    // );

    match (expected, actual) {
        (expected, actual) if expected.is_same(actual) => {
            match (ownership, expression.infer_owned(ctx, scope, errors)) {
                (
                    Ownership::SharedOwned | Ownership::UniqueOwned,
                    Ownership::SharedOwned | Ownership::Borrowed | Ownership::MutBorrowed,
                ) => {
                    transpile!(ctx, scope, errors, "{}.to_owned()", expression)
                }
                (Ownership::Borrowed, Ownership::UniqueOwned | Ownership::SharedOwned) => {
                    transpile!(ctx, scope, errors, "&{}", expression)
                }
                _ => expression.transpile(ctx, scope, errors),
            }
        }
        (_, TypeElement::Infer(_)) => match (ownership, expression.infer_owned(ctx, scope, errors)) {
            (
                Ownership::SharedOwned | Ownership::UniqueOwned,
                Ownership::SharedOwned | Ownership::Borrowed | Ownership::MutBorrowed,
            ) => {
                transpile!(ctx, scope, errors, "{}.to_owned()", expression)
            }
            (Ownership::Borrowed, Ownership::UniqueOwned | Ownership::SharedOwned) => {
                transpile!(ctx, scope, errors, "&{}", expression)
            }
            _ => expression.transpile(ctx, scope, errors),
        },
        (_, TypeElement::Never(_) | TypeElement::Void(_)) => {
            expression.transpile(ctx, scope, errors)
        }
        (TypeElement::Void(_) | TypeElement::Infer(_), _) => {
            expression.transpile(ctx, scope, errors)
        }
        (TypeElement::Optional(some), actual)
            if some.inner.is_same(actual) || actual.is_number() =>
        {
            let postfix = match expression.infer_owned(ctx, scope, errors) {
                // TODO: we want to distinguish between Owned and SharedOwned, the latter needs to be cloned
                Ownership::Borrowed | Ownership::MutBorrowed => ".to_owned()",
                _ => {
                    if actual.is_number() {
                        ""
                    } else {
                        ".to_owned()"
                    }
                }
            };
            transpile!(ctx, scope, errors, "Some({}{postfix})", expression)
        }
        // Handle __Number type being cast to Optional
        (TypeElement::Optional(_), actual) if actual.is_number() => {
            transpile!(ctx, scope, errors, "Some({})", expression)
        }
        (TypeElement::Result(res), actual) if res.success.is_same(actual) || actual.is_number() => {
            let postfix = match expression.infer_owned(ctx, scope, errors) {
                Ownership::Borrowed | Ownership::MutBorrowed => ".to_owned()",
                _ => {
                    if actual.is_number() {
                        ""
                    } else {
                        ".to_owned()"
                    }
                }
            };
            transpile!(ctx, scope, errors, "Ok({}{postfix})", expression)
        }
        // Handle __Number type being cast to Result
        (TypeElement::Result(_), actual) if actual.is_number() => {
            transpile!(ctx, scope, errors, "Ok({})", expression)
        }
        (TypeElement::Result(res), actual)
            if res
                .error
                .as_ref()
                .is_some_and(|inner| inner.is_same(actual)) =>
        {
            // TODO: This should not be autocast but instead require a "throw" keyword
            transpile!(ctx, scope, errors, "Err({})", expression)
        }
        (TypeElement::Result(_), actual) => {
            errors.warning(
                format!("Wrapping non-matching type {} in Ok", actual),
                None
            );
            let postfix = match expression.infer_owned(ctx, scope, errors) {
                Ownership::Borrowed | Ownership::MutBorrowed => ".to_owned()",
                _ => ".to_owned()",
            };
            transpile!(
                ctx,
                scope,
                errors,
                "/*non-matching*/Ok({}{postfix})",
                expression
            )
        }
        (TypeElement::Optional(_), TypeElement::Optional(_)) => {
            errors.warning(
                format!("Wrapping non-matching type {} in Some", actual),
                None
            );
            transpile!(ctx, scope, errors, "/*non-matching*/{}", expression)
        }
        (TypeElement::Optional(_), actual) => {
            errors.warning(
                format!("Wrapping non-matching type {} in Some", actual),
                None
            );
            let postfix = match expression.infer_owned(ctx, scope, errors) {
                Ownership::Borrowed | Ownership::MutBorrowed => ".to_owned()",
                _ => ".to_owned()",
            };
            transpile!(
                ctx,
                scope,
                errors,
                "/*non-matching*/Some({}{postfix})",
                expression
            )
        }
        // TODO: only allow this for expected number types
        (_, TypeElement::Plain(BasicTypeItem { ident, span: _ }))
            if ident.as_str() == "__Number" =>
        {
            expression.transpile(ctx, scope, errors)
        }
        (_, _) => {
            // For unknown type conversions, apply basic ownership conversion
            match ownership {
                Ownership::Borrowed => {
                    // If we need a borrowed value, try to borrow the expression
                    transpile!(ctx, scope, errors, "&{}", expression)
                }
                _ => {
                    // For other ownership types, just return the expression as-is
                    expression.transpile(ctx, scope, errors)
                }
            }
        }
    }
}
