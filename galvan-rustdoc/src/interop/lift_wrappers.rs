use rustdoc_types::{Crate, Path, Type};

use galvan_ast::{
    ArrayTypeItem, DictionaryTypeItem, OptionalTypeItem, OrderedDictionaryTypeItem,
    ParametricTypeItem, SetTypeItem, Span, TypeElement, TypeIdent,
};

use super::lift_model::LiftedType;
use super::lift_type::{plain_type, result_type, string_type};
use super::rustdoc_json::resolved_type_args;
use super::rustdoc_path::{
    resolved_path_is_unqualified_or_in_crates, resolved_path_is_unqualified_or_matches_any,
    resolved_path_matches, resolved_type_name,
};
use super::RustInterop;

impl RustInterop {
    pub(super) fn lift_known_resolved_type(
        &mut self,
        krate: &Crate,
        name: &str,
        resolved: &Path,
        args: &[LiftedType],
    ) -> Option<LiftedType> {
        match classify_wrapper(krate, name, resolved)? {
            WrapperShape::String => Some(LiftedType::new(string_type())),
            WrapperShape::Option => Some(LiftedType::new(TypeElement::Optional(Box::new(
                OptionalTypeItem {
                    inner: args.first()?.ty.clone(),
                    span: Span::default(),
                },
            )))),
            WrapperShape::ResultSingle => Some(result_type(args.first()?, None)),
            WrapperShape::ResultDouble => Some(result_type(
                args.first()?,
                args.get(1)
                    .map(|arg| arg.ty.clone())
                    .or_else(|| Some(plain_type(TypeIdent::new("__UnknownRustError")))),
            )),
            WrapperShape::Array => Some(LiftedType::new(TypeElement::Array(Box::new(
                ArrayTypeItem {
                    elements: args.first()?.ty.clone(),
                    span: Span::default(),
                },
            )))),
            WrapperShape::Set => Some(LiftedType::new(TypeElement::Set(Box::new(SetTypeItem {
                elements: args.first()?.ty.clone(),
                span: Span::default(),
            })))),
            WrapperShape::Dictionary => Some(LiftedType::new(TypeElement::Dictionary(Box::new(
                DictionaryTypeItem {
                    key: args.first()?.ty.clone(),
                    value: args.get(1)?.ty.clone(),
                    span: Span::default(),
                },
            )))),
            WrapperShape::OrderedDictionary => Some(LiftedType::new(
                TypeElement::OrderedDictionary(Box::new(OrderedDictionaryTypeItem {
                    key: args.first()?.ty.clone(),
                    value: args.get(1)?.ty.clone(),
                    span: Span::default(),
                })),
            )),
            // Bare locks are recognized as known wrappers (so incomplete metadata is
            // treated as unliftable) but are intentionally not lifted here.
            WrapperShape::Lock => None,
        }
    }

    pub(super) fn lift_arc_type_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        resolved: &Path,
    ) -> Option<LiftedType> {
        let inner = resolved_type_args(resolved).into_iter().next()?;
        if let Some(shared) = self.lift_arc_shared_inner(krate, crate_name, inner) {
            return Some(shared);
        }

        let inner = self.lift_type_from_json(krate, crate_name, inner)?;
        let name = resolved_type_name(resolved)?;
        self.push_resolved_type(krate, crate_name, name.as_ref(), resolved);
        Some(LiftedType::new(TypeElement::Parametric(
            ParametricTypeItem {
                base_type: TypeIdent::new(name.as_ref()),
                type_args: vec![inner.ty],
                span: Span::default(),
            },
        )))
    }

    fn lift_arc_shared_inner(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        inner: &Type,
    ) -> Option<LiftedType> {
        let Type::ResolvedPath(resolved) = inner else {
            return None;
        };
        let name = resolved_type_name(resolved)?;
        if !resolved_path_is_unqualified_or_in_crates(krate, resolved, &["std", "core", "alloc"]) {
            return None;
        }
        if name.as_ref() == "Mutex" {
            return self.lift_lock_type_from_json(krate, crate_name, resolved);
        }
        None
    }

    pub(super) fn lift_lock_type_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        resolved: &Path,
    ) -> Option<LiftedType> {
        let arg = resolved_type_args(resolved).into_iter().next()?;
        let inner = self.lift_type_from_json(krate, crate_name, arg)?;
        Some(LiftedType::with_modifier(
            inner.ty,
            galvan_ast::DeclModifier::Ref,
        ))
    }
}

pub(super) fn known_lifted_resolved_type(krate: &Crate, name: &str, resolved: &Path) -> bool {
    classify_wrapper(krate, name, resolved).is_some()
}

/// The shape a known wrapper type lifts to. This is the single source of truth for
/// which `(name, resolved path)` pairs are recognized wrappers; both the lifter
/// (`lift_known_resolved_type`) and the predicate (`known_lifted_resolved_type`)
/// consult it, so their name/path membership can never drift apart.
enum WrapperShape {
    String,
    Option,
    /// Single-payload result (`FlexResult`, `anyhow::Result`).
    ResultSingle,
    /// Standard two-parameter `Result`.
    ResultDouble,
    Array,
    Set,
    Dictionary,
    OrderedDictionary,
    /// `Mutex`/`RwLock`: recognized but not lifted (bare locks stay unliftable).
    Lock,
}

fn classify_wrapper(krate: &Crate, name: &str, resolved: &Path) -> Option<WrapperShape> {
    let standard_wrapper =
        resolved_path_is_unqualified_or_in_crates(krate, resolved, &["std", "core", "alloc"]);
    let flex_result = resolved_path_is_unqualified_or_matches_any(
        krate,
        resolved,
        &[&["galvan", "std", "FlexResult"]],
    );

    match name {
        "String" if standard_wrapper => Some(WrapperShape::String),
        "Option" if standard_wrapper => Some(WrapperShape::Option),
        "FlexResult" if flex_result => Some(WrapperShape::ResultSingle),
        "Result" if resolved_path_matches(krate, resolved, &["anyhow", "Result"]) => {
            Some(WrapperShape::ResultSingle)
        }
        "Result" if standard_wrapper => Some(WrapperShape::ResultDouble),
        "Vec" if standard_wrapper => Some(WrapperShape::Array),
        "HashSet" if standard_wrapper => Some(WrapperShape::Set),
        "HashMap" if standard_wrapper => Some(WrapperShape::Dictionary),
        "BTreeMap" if standard_wrapper => Some(WrapperShape::OrderedDictionary),
        "Mutex" if standard_wrapper => Some(WrapperShape::Lock),
        _ => None,
    }
}
