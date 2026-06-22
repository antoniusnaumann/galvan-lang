use galvan_ast::{Ownership, TypeElement};

use crate::builtins::{CheckBuiltins, IsSame};
use crate::hir::{Adjustment, ConcatKind, HirExpression, HirExpressionKind, HirLiteral};

use super::Checker;

/// What the surrounding context expects of an expression.
///
/// This replaces the implicit threading of `Scope::return_type` and
/// `Scope::ownership` through ad-hoc child scopes in the old transpiler.
#[derive(Clone, Debug)]
pub(crate) struct Expected {
    pub ty: TypeElement,
    pub ownership: Ownership,
}

impl Expected {
    /// No expectation: the expression is used as-is and never coerced
    pub fn free() -> Self {
        Self {
            ty: TypeElement::infer(),
            ownership: Ownership::UniqueOwned,
        }
    }

    /// The context consumes an owned value of the given type
    pub fn owned(ty: TypeElement) -> Self {
        Self {
            ty,
            ownership: Ownership::UniqueOwned,
        }
    }

    pub fn with(ty: TypeElement, ownership: Ownership) -> Self {
        Self { ty, ownership }
    }

    pub fn void() -> Self {
        Self {
            ty: TypeElement::void(),
            ownership: Ownership::UniqueOwned,
        }
    }

    pub fn is_free(&self) -> bool {
        self.ty.is_infer()
    }

    pub fn is_void(&self) -> bool {
        self.ty.is_void()
    }
}

/// Checks whether two types are compatible, treating `Infer`, generic type
/// parameters and the `__Number` intrinsic as wildcards (recursively)
pub fn types_compatible(expected: &TypeElement, actual: &TypeElement) -> bool {
    match (expected, actual) {
        (TypeElement::Infer(_), _) | (_, TypeElement::Infer(_)) => true,
        (TypeElement::Never(_), _) | (_, TypeElement::Never(_)) => true,
        // Generic type parameters unify with any type
        (TypeElement::Generic(_), _) | (_, TypeElement::Generic(_)) => true,
        (expected, actual) if expected.is_number() || actual.is_number() => {
            // The number intrinsic is compatible with every plain type
            // (rustc decides whether the literal actually fits)
            matches!(expected, TypeElement::Plain(_)) && matches!(actual, TypeElement::Plain(_))
        }
        (TypeElement::Array(a), TypeElement::Array(b)) => {
            types_compatible(&a.elements, &b.elements)
        }
        (TypeElement::Set(a), TypeElement::Set(b)) => types_compatible(&a.elements, &b.elements),
        (TypeElement::Dictionary(a), TypeElement::Dictionary(b)) => {
            types_compatible(&a.key, &b.key) && types_compatible(&a.value, &b.value)
        }
        (TypeElement::OrderedDictionary(a), TypeElement::OrderedDictionary(b)) => {
            types_compatible(&a.key, &b.key) && types_compatible(&a.value, &b.value)
        }
        (TypeElement::Tuple(a), TypeElement::Tuple(b)) => {
            a.elements.len() == b.elements.len()
                && a.elements
                    .iter()
                    .zip(&b.elements)
                    .all(|(a, b)| types_compatible(a, b))
        }
        (TypeElement::Optional(a), TypeElement::Optional(b)) => {
            types_compatible(&a.inner, &b.inner)
        }
        (TypeElement::Result(a), TypeElement::Result(b)) => {
            types_compatible(&a.success, &b.success)
                && match (&a.error, &b.error) {
                    (Some(a), Some(b)) => types_compatible(a, b),
                    _ => true,
                }
        }
        (TypeElement::Closure(a), TypeElement::Closure(b)) => {
            a.parameters.len() == b.parameters.len()
                && a.parameters
                    .iter()
                    .zip(&b.parameters)
                    .all(|(a, b)| types_compatible(a, b))
                && types_compatible(&a.return_ty, &b.return_ty)
        }
        (expected, actual) => expected.is_same(actual),
    }
}

/// Classifies a `++` concatenation by its operand types: appending a single
/// element, merging a collection of the same shape, or stringifying the
/// right-hand side (strings only)
pub(crate) fn concat_kind(lhs: &TypeElement, rhs: &TypeElement) -> ConcatKind {
    match lhs {
        TypeElement::Array(array) => {
            if matches!(rhs, TypeElement::Array(_))
                && !value_matches_concrete_element(&array.elements, rhs)
            {
                ConcatKind::Collection
            } else {
                ConcatKind::Element
            }
        }
        TypeElement::Set(set) => {
            if matches!(rhs, TypeElement::Set(_))
                && !value_matches_concrete_element(&set.elements, rhs)
            {
                ConcatKind::Collection
            } else {
                ConcatKind::Element
            }
        }
        TypeElement::Plain(basic) if basic.ident.as_str() == "String" => match rhs {
            TypeElement::Plain(rhs) if rhs.ident.as_str() == "Char" => ConcatKind::Element,
            TypeElement::Plain(rhs) if rhs.ident.as_str() == "String" => ConcatKind::Collection,
            _ => ConcatKind::Stringify,
        },
        _ => ConcatKind::Collection,
    }
}

/// `true` when the value type matches a concrete (non-wildcard) element type,
/// meaning a collection-typed right-hand side is appended as a single element
/// (e.g. pushing a `[Int]` row into a `[[Int]]` matrix)
fn value_matches_concrete_element(element_ty: &TypeElement, value_ty: &TypeElement) -> bool {
    !matches!(element_ty, TypeElement::Infer(_) | TypeElement::Generic(_))
        && types_compatible(element_ty, value_ty)
}

/// The element type a `++` element append consumes for the given collection
fn concat_element_type(lhs: &TypeElement) -> TypeElement {
    match lhs {
        TypeElement::Array(array) => array.elements.clone(),
        TypeElement::Set(set) => set.elements.clone(),
        TypeElement::Plain(basic) if basic.ident.as_str() == "String" => {
            TypeElement::Plain(galvan_ast::BasicTypeItem {
                ident: galvan_ast::TypeIdent::new("Char"),
                span: galvan_ast::Span::default(),
            })
        }
        _ => TypeElement::infer(),
    }
}

impl Checker<'_> {
    /// Coerces an expression to the given expectation by appending
    /// [`Adjustment`]s. This is the single place where ownership and
    /// type-wrapping decisions are made.
    pub(crate) fn coerce(&mut self, expr: HirExpression, expected: &Expected) -> HirExpression {
        // No expectation or statement context: leave the expression untouched
        if expected.ty.is_infer() || expected.ty.is_void() {
            return expr;
        }
        // Diverging or value-less expressions cannot be adjusted
        if matches!(expr.ty, TypeElement::Never(_) | TypeElement::Void(_)) {
            return expr;
        }

        match (&expected.ty, &expr.ty) {
            // Unknown actual type: only reconcile ownership
            (_, TypeElement::Infer(_)) => self.adjust_ownership(expr, expected.ownership),
            (expected_ty, actual) if types_compatible(expected_ty, actual) => {
                self.adjust_ownership(expr, expected.ownership)
            }
            // Auto-wrap values in `Some` when an optional is expected
            (TypeElement::Optional(some), actual)
                if types_compatible(&some.inner, actual) || actual.is_number() =>
            {
                let expr = self.ensure_owned(expr);
                expr.adjusted(Adjustment::WrapSome)
            }
            // Auto-wrap success values in `Ok` when a result is expected
            (TypeElement::Result(res), actual)
                if types_compatible(&res.success, actual) || actual.is_number() =>
            {
                let expr = self.ensure_owned(expr);
                expr.adjusted(Adjustment::WrapOk)
            }
            // Auto-wrap error values in `Err`
            // TODO: This should not be autocast but instead require a "throw" keyword
            (TypeElement::Result(res), actual)
                if res
                    .error
                    .as_ref()
                    .is_some_and(|error| types_compatible(error, actual)) =>
            {
                expr.adjusted(Adjustment::WrapErr)
            }
            (TypeElement::Result(_), actual) => {
                self.errors
                    .warning(format!("Wrapping non-matching type {} in Ok", actual), None);
                let expr = self.ensure_owned(expr);
                expr.adjusted(Adjustment::WrapOk)
            }
            (TypeElement::Optional(_), actual) => {
                self.errors.warning(
                    format!("Wrapping non-matching type {} in Some", actual),
                    None,
                );
                let expr = self.ensure_owned(expr);
                expr.adjusted(Adjustment::WrapSome)
            }
            (_, actual) if actual.is_number() => expr,
            (expected_ty, actual) => {
                self.errors.warning(
                    format!("Type mismatch: expected {}, found {}", expected_ty, actual),
                    Some(expr.span.into()),
                );
                self.adjust_ownership(expr, expected.ownership)
            }
        }
    }

    /// Reconciles the ownership of an already type-correct expression with
    /// what the context expects
    pub(crate) fn adjust_ownership(
        &mut self,
        expr: HirExpression,
        expected: Ownership,
    ) -> HirExpression {
        use Ownership::*;

        let actual = expr.adjusted_ownership();
        match (expected, actual) {
            // The context consumes the value
            (UniqueOwned | SharedOwned, UniqueOwned) => expr,
            (UniqueOwned | SharedOwned, SharedOwned | Borrowed | MutBorrowed) => {
                if self.is_copy(&expr.ty) {
                    expr
                } else {
                    expr.adjusted(Adjustment::ToOwned)
                }
            }
            (UniqueOwned | SharedOwned, Ref) => expr
                .adjusted(Adjustment::LockRef)
                .adjusted(Adjustment::ToOwned),

            // The context borrows the value
            (Borrowed, UniqueOwned | SharedOwned) => expr.adjusted(Adjustment::Borrow),
            (Borrowed, Borrowed | MutBorrowed) => expr,
            (Borrowed, Ref) => expr.adjusted(Adjustment::LockRef),

            // The context mutably borrows the value
            (MutBorrowed, UniqueOwned | SharedOwned) => expr.adjusted(Adjustment::MutBorrow),
            (MutBorrowed, MutBorrowed) => expr,
            (MutBorrowed, Borrowed) => {
                self.errors.warning(
                    "Cannot mutably borrow an immutably borrowed value".to_string(),
                    Some(expr.span.into()),
                );
                expr
            }
            (MutBorrowed, Ref) => expr
                .adjusted(Adjustment::LockRef)
                .adjusted(Adjustment::Deref)
                .adjusted(Adjustment::MutBorrow),

            // `ref` declarations wrap the initializer at the declaration site,
            // `ref` arguments are wrapped with Arc::clone at the call site
            (Ref, _) => expr,
        }
    }

    /// Coerces the right-hand side of a `++` to the ownership its generated
    /// shape consumes: appended elements are owned (`push`/`insert` consume
    /// them) and arrays extended in place own their argument (`extend`
    /// iterates by value). Merged collections and stringified values are
    /// borrowed or cloned inside the generated pattern instead.
    pub(crate) fn coerce_concat_value(
        &mut self,
        lhs_ty: &TypeElement,
        kind: ConcatKind,
        value: HirExpression,
        extends_in_place: bool,
    ) -> HirExpression {
        match kind {
            ConcatKind::Element => {
                let element_ty = concat_element_type(lhs_ty);
                self.coerce(value, &Expected::owned(element_ty))
            }
            ConcatKind::Collection
                if extends_in_place && matches!(lhs_ty, TypeElement::Array(_)) =>
            {
                self.ensure_owned(value)
            }
            ConcatKind::Collection | ConcatKind::Stringify => value,
        }
    }

    /// Makes sure the value can be stored (inside `Some(...)`/`Ok(...)` or a
    /// collection) by cloning borrowed non-copy values
    pub(crate) fn ensure_owned(&mut self, expr: HirExpression) -> HirExpression {
        use Ownership::*;
        match expr.adjusted_ownership() {
            UniqueOwned => expr,
            SharedOwned | Borrowed | MutBorrowed => {
                if self.is_copy(&expr.ty) {
                    expr
                } else {
                    expr.adjusted(Adjustment::ToOwned)
                }
            }
            Ref => expr
                .adjusted(Adjustment::LockRef)
                .adjusted(Adjustment::ToOwned),
        }
    }

    /// Coercion used for arguments to functions with *unknown* signatures
    /// (e.g. methods of the Rust standard library). Since no parameter types
    /// are available, this uses a heuristic over the expression kind:
    /// temporaries produced by calls and string literals are borrowed, shared
    /// locals are borrowed, everything else is passed through.
    pub(crate) fn coerce_unknown_argument(&mut self, expr: HirExpression) -> HirExpression {
        use Ownership::*;

        match &expr.kind {
            HirExpressionKind::Variable(_) => match expr.ownership {
                SharedOwned => expr.adjusted(Adjustment::Borrow),
                Ref => expr.adjusted(Adjustment::LockRef),
                UniqueOwned | Borrowed | MutBorrowed => expr,
            },
            HirExpressionKind::Closure(_) => expr,
            HirExpressionKind::FunctionCall(_)
            | HirExpressionKind::MethodCall(_)
            | HirExpressionKind::Literal(HirLiteral::String(_)) => {
                expr.adjusted(Adjustment::Borrow)
            }
            _ => expr,
        }
    }
}
