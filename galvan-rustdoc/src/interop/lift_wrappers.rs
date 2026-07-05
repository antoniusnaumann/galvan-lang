use serde_json::Value;

use galvan_ast::{
    ArrayTypeItem, DictionaryTypeItem, OptionalTypeItem, OrderedDictionaryTypeItem,
    ParametricTypeItem, SetTypeItem, Span, TypeElement, TypeIdent,
};

use super::lift_model::LiftedType;
use super::lift_type::{
    atomic_type, plain_type, resolved_path_is_unqualified_or_in_crates,
    resolved_path_is_unqualified_or_matches_any, resolved_path_matches, result_type, string_type,
};
use super::rustdoc_json::resolved_type_args;
use super::rustdoc_path::resolved_type_name;
use super::RustInterop;

impl RustInterop {
    pub(super) fn lift_known_resolved_type(
        &mut self,
        name: &str,
        resolved: &Value,
        args: &[LiftedType],
    ) -> Option<LiftedType> {
        match classify_wrapper(name, resolved)? {
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
            WrapperShape::Set => {
                Some(LiftedType::new(TypeElement::Set(Box::new(SetTypeItem {
                    elements: args.first()?.ty.clone(),
                    span: Span::default(),
                }))))
            }
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
        crate_name: &str,
        resolved: &Value,
    ) -> Option<LiftedType> {
        let inner = resolved_type_args(resolved).into_iter().next()?;
        if let Some(shared) = self.lift_arc_shared_inner(crate_name, inner) {
            return Some(shared);
        }

        let inner = self.lift_type_from_json(crate_name, inner)?;
        let name = resolved_type_name(resolved)?;
        self.push_resolved_type(crate_name, name.as_ref(), resolved);
        Some(LiftedType::new(TypeElement::Parametric(
            ParametricTypeItem {
                base_type: TypeIdent::new(name.as_ref()),
                type_args: vec![inner.ty],
                span: Span::default(),
            },
        )))
    }

    fn lift_arc_shared_inner(&mut self, crate_name: &str, inner: &Value) -> Option<LiftedType> {
        let resolved = inner.get("resolved_path")?;
        let name = resolved_type_name(resolved)?;
        if !resolved_path_is_unqualified_or_in_crates(resolved, &["std", "core", "alloc"]) {
            return None;
        }
        if matches!(name.as_ref(), "Mutex" | "RwLock") {
            return self.lift_lock_type_from_json(crate_name, resolved);
        }

        atomic_type(name.as_ref())
            .map(|ty| LiftedType::with_modifier(ty, galvan_ast::DeclModifier::Ref))
    }

    pub(super) fn lift_lock_type_from_json(
        &mut self,
        crate_name: &str,
        resolved: &Value,
    ) -> Option<LiftedType> {
        let arg = resolved_type_args(resolved).into_iter().next()?;
        let inner = self.lift_type_from_json(crate_name, arg)?;
        Some(LiftedType::with_modifier(
            inner.ty,
            galvan_ast::DeclModifier::Ref,
        ))
    }
}

pub(super) fn known_lifted_resolved_type(name: &str, resolved: &Value) -> bool {
    classify_wrapper(name, resolved).is_some()
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

fn classify_wrapper(name: &str, resolved: &Value) -> Option<WrapperShape> {
    let standard_wrapper =
        resolved_path_is_unqualified_or_in_crates(resolved, &["std", "core", "alloc"]);
    let indexmap_wrapper = resolved_path_is_unqualified_or_in_crates(resolved, &["indexmap"]);
    let flex_result =
        resolved_path_is_unqualified_or_matches_any(resolved, &[&["galvan", "std", "FlexResult"]]);

    match name {
        "String" if standard_wrapper => Some(WrapperShape::String),
        "Option" if standard_wrapper => Some(WrapperShape::Option),
        "FlexResult" if flex_result => Some(WrapperShape::ResultSingle),
        "Result" if resolved_path_matches(resolved, &["anyhow", "Result"]) => {
            Some(WrapperShape::ResultSingle)
        }
        "Result" if standard_wrapper => Some(WrapperShape::ResultDouble),
        "Vec" | "VecDeque" | "LinkedList" if standard_wrapper => Some(WrapperShape::Array),
        "HashSet" | "BTreeSet" if standard_wrapper => Some(WrapperShape::Set),
        "IndexSet" if indexmap_wrapper => Some(WrapperShape::Set),
        "HashMap" if standard_wrapper => Some(WrapperShape::Dictionary),
        "BTreeMap" if standard_wrapper => Some(WrapperShape::OrderedDictionary),
        "IndexMap" if indexmap_wrapper => Some(WrapperShape::OrderedDictionary),
        "Mutex" | "RwLock" if standard_wrapper => Some(WrapperShape::Lock),
        _ => None,
    }
}
