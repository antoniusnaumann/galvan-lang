//! Code generation from the typed HIR.
//!
//! All type and ownership decisions were made by the typechecker and are
//! stored in the HIR; code generation renders nodes mechanically and applies
//! the stored [`Adjustment`](galvan_hir::hir::Adjustment)s.

mod expression;
mod function;
mod statement;

pub(crate) use function::{
    transpile_function, transpile_main, transpile_signature, transpile_test,
};

use galvan_ast::TypeElement;
use galvan_hir::hir::{Adjustment, HirExpression, HirExpressionKind};

use crate::context::Context;
use crate::ErrorCollector;
use crate::Transpile;

impl Transpile for HirExpression {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let rendered = self.kind.transpile(ctx, errors);
        apply_adjustments(rendered, &self.kind, &self.adjustments)
    }
}

pub(crate) fn ref_storage_type(ty: &TypeElement, rendered: String) -> String {
    atomic_ref_storage_type(ty)
        .map(|atomic| format!("std::sync::Arc<std::sync::atomic::{atomic}>"))
        .unwrap_or_else(|| format!("std::sync::Arc<std::sync::Mutex<{rendered}>>"))
}

pub(crate) fn wrap_ref_storage_value(
    rendered: String,
    value: &HirExpression,
    storage_ty: &TypeElement,
) -> String {
    if value.adjustments.last() == Some(&Adjustment::ArcClone) {
        rendered
    } else if let Some(atomic) = atomic_ref_storage_type(storage_ty) {
        format!("std::sync::Arc::new(std::sync::atomic::{atomic}::new({rendered}))")
    } else {
        format!("(&({rendered})).__to_ref()")
    }
}

fn atomic_ref_storage_type(ty: &TypeElement) -> Option<&'static str> {
    let TypeElement::Plain(plain) = ty else {
        return None;
    };

    match plain.ident.as_str() {
        "Bool" => Some("AtomicBool"),
        "I8" => Some("AtomicI8"),
        "I16" => Some("AtomicI16"),
        "I32" => Some("AtomicI32"),
        "I64" | "Int" => Some("AtomicI64"),
        "ISize" => Some("AtomicIsize"),
        "U8" => Some("AtomicU8"),
        "U16" => Some("AtomicU16"),
        "U32" => Some("AtomicU32"),
        "U64" => Some("AtomicU64"),
        "USize" => Some("AtomicUsize"),
        _ => None,
    }
}

/// Renders the coercions determined by the typechecker around an expression
fn apply_adjustments(
    rendered: String,
    kind: &HirExpressionKind,
    adjustments: &[Adjustment],
) -> String {
    let mut result = rendered;
    for (i, adjustment) in adjustments.iter().enumerate() {
        // Once the first adjustment is applied, the rendered code is a call,
        // wrap or reference that needs no further parenthesization
        let parenthesize = i == 0 && needs_parens(kind);
        result = match adjustment {
            Adjustment::Borrow if parenthesize => format!("&({result})"),
            Adjustment::Borrow => format!("&{result}"),
            Adjustment::MutBorrow if parenthesize => format!("&mut ({result})"),
            Adjustment::MutBorrow => format!("&mut {result}"),
            Adjustment::Deref => format!("*{result}"),
            Adjustment::ToOwned if parenthesize => format!("({result}).to_owned()"),
            Adjustment::ToOwned => format!("{result}.to_owned()"),
            Adjustment::WrapSome => format!("Some({result})"),
            Adjustment::WrapOk => format!("Ok({result})"),
            Adjustment::WrapErr => format!("Err({result})"),
            Adjustment::LockRef => format!("{result}.lock().unwrap()"),
            Adjustment::ArcClone => format!("::std::sync::Arc::clone(&{result})"),
        };
    }
    result
}

/// Whether prefix or postfix adjustments need to parenthesize this kind of
/// expression to preserve precedence
fn needs_parens(kind: &HirExpressionKind) -> bool {
    use HirExpressionKind::*;
    match kind {
        Variable(_) | Literal(_) | FunctionCall(_) | MethodCall(_) | FieldAccess(_)
        | SafeAccess(_) | ConstructorCall(_) | EnumAccess(_) | EnumConstructor(_)
        | Collection(_) | Index(_) | Group(_) | Yeet(_) | Print(_) | Assert(_) | Error(_) => false,
        If(_) | ElseUnwrap(_) | Try(_) | For(_) | Match(_) | Closure(_) | Logical(_)
        | Arithmetic(_) | Bitwise(_) | Comparison(_) | CollectionOp(_) | Range(_) => true,
    }
}
