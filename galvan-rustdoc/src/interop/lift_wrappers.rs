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
        let standard_wrapper =
            resolved_path_is_unqualified_or_in_crates(resolved, &["std", "core", "alloc"]);
        let indexmap_wrapper = resolved_path_is_unqualified_or_in_crates(resolved, &["indexmap"]);
        let flex_result = resolved_path_is_unqualified_or_matches_any(
            resolved,
            &[&["galvan", "std", "FlexResult"]],
        );

        match name {
            "String" if standard_wrapper => Some(LiftedType::new(string_type())),
            "Option" if standard_wrapper => Some(LiftedType::new(TypeElement::Optional(Box::new(
                OptionalTypeItem {
                    inner: args.first()?.ty.clone(),
                    span: Span::default(),
                },
            )))),
            "FlexResult" if flex_result => Some(result_type(args.first()?, None)),
            "Result" if resolved_path_matches(resolved, &["anyhow", "Result"]) => {
                Some(result_type(args.first()?, None))
            }
            "Result" if standard_wrapper => Some(result_type(
                args.first()?,
                args.get(1)
                    .map(|arg| arg.ty.clone())
                    .or_else(|| Some(plain_type(TypeIdent::new("__UnknownRustError")))),
            )),
            "Vec" | "VecDeque" | "LinkedList" if standard_wrapper => Some(LiftedType::new(
                TypeElement::Array(Box::new(ArrayTypeItem {
                    elements: args.first()?.ty.clone(),
                    span: Span::default(),
                })),
            )),
            "HashSet" | "BTreeSet" if standard_wrapper => {
                Some(LiftedType::new(TypeElement::Set(Box::new(SetTypeItem {
                    elements: args.first()?.ty.clone(),
                    span: Span::default(),
                }))))
            }
            "IndexSet" if indexmap_wrapper => {
                Some(LiftedType::new(TypeElement::Set(Box::new(SetTypeItem {
                    elements: args.first()?.ty.clone(),
                    span: Span::default(),
                }))))
            }
            "HashMap" if standard_wrapper => Some(LiftedType::new(TypeElement::Dictionary(
                Box::new(DictionaryTypeItem {
                    key: args.first()?.ty.clone(),
                    value: args.get(1)?.ty.clone(),
                    span: Span::default(),
                }),
            ))),
            "BTreeMap" if standard_wrapper => Some(LiftedType::new(
                TypeElement::OrderedDictionary(Box::new(OrderedDictionaryTypeItem {
                    key: args.first()?.ty.clone(),
                    value: args.get(1)?.ty.clone(),
                    span: Span::default(),
                })),
            )),
            "IndexMap" if indexmap_wrapper => Some(LiftedType::new(
                TypeElement::OrderedDictionary(Box::new(OrderedDictionaryTypeItem {
                    key: args.first()?.ty.clone(),
                    value: args.get(1)?.ty.clone(),
                    span: Span::default(),
                })),
            )),
            _ => None,
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
    let standard_wrapper =
        resolved_path_is_unqualified_or_in_crates(resolved, &["std", "core", "alloc"]);
    let indexmap_wrapper = resolved_path_is_unqualified_or_in_crates(resolved, &["indexmap"]);
    let flex_result =
        resolved_path_is_unqualified_or_matches_any(resolved, &[&["galvan", "std", "FlexResult"]]);

    matches!(
        name,
        "String"
            | "Option"
            | "Result"
            | "Vec"
            | "VecDeque"
            | "LinkedList"
            | "HashSet"
            | "BTreeSet"
            | "HashMap"
            | "BTreeMap" if standard_wrapper
    ) || matches!(name, "IndexSet" | "IndexMap" if indexmap_wrapper)
        || matches!(name, "FlexResult" if flex_result)
        || resolved_path_matches(resolved, &["anyhow", "Result"])
}
