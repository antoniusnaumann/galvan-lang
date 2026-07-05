use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use galvan_ast::{Ident, ToplevelItem, TypeElement, TypeIdent, UseDecl};

use super::function_id::RustFunctionId;
use super::RustInterop;

impl RustInterop {
    pub fn import_uses(&mut self, uses: &[ToplevelItem<UseDecl>]) {
        self.by_imported_type.clear();
        self.by_imported_function.clear();
        self.by_imported_constant.clear();

        let mut ambiguous_types = HashSet::new();
        let mut ambiguous_functions = HashSet::new();
        let mut ambiguous_constants = HashSet::new();

        for use_decl in uses {
            let Some(namespace) = use_decl.path.segments.first() else {
                continue;
            };
            let namespace = namespace.as_str();
            match use_decl.path.segments.as_slice() {
                [_] => self.import_namespace(
                    namespace,
                    &mut ambiguous_types,
                    &mut ambiguous_functions,
                    &mut ambiguous_constants,
                ),
                [_, item] => self.import_item(
                    namespace,
                    item.as_str(),
                    &mut ambiguous_types,
                    &mut ambiguous_functions,
                    &mut ambiguous_constants,
                ),
                _ => {}
            }
        }
    }

    fn import_namespace(
        &mut self,
        namespace: &str,
        ambiguous_types: &mut HashSet<TypeIdent>,
        ambiguous_functions: &mut HashSet<(String, RustFunctionId)>,
        ambiguous_constants: &mut HashSet<Ident>,
    ) {
        for (idx, ty) in self.types.iter().enumerate() {
            if ty.namespace.as_ref() != namespace {
                continue;
            }
            insert_unambiguous(
                &mut self.by_imported_type,
                ambiguous_types,
                ty.name.clone(),
                idx,
            );
        }
        for (idx, function) in self.functions.iter().enumerate() {
            if function.namespace.as_ref() != namespace {
                continue;
            }
            let signature = &function.decl.item.signature;
            let labels = signature.overload_labels();
            let labels = labels
                .iter()
                .map(|label| label.as_str())
                .collect::<Vec<_>>();
            let receiver = signature
                .receiver()
                .and_then(|param| match &param.param_type {
                    TypeElement::Plain(plain) => Some(&plain.ident),
                    TypeElement::Parametric(parametric) => Some(&parametric.base_type),
                    _ => None,
                });
            let id = RustFunctionId::new(receiver, signature.identifier.as_str(), &labels);
            insert_unambiguous(
                &mut self.by_imported_function,
                ambiguous_functions,
                ("".to_string(), id),
                idx,
            );
        }
        for (idx, constant) in self.constants.iter().enumerate() {
            if constant.namespace.as_ref() != namespace || constant.associated_receiver.is_some() {
                continue;
            }
            insert_unambiguous(
                &mut self.by_imported_constant,
                ambiguous_constants,
                constant.name.clone(),
                idx,
            );
        }
    }

    fn import_item(
        &mut self,
        namespace: &str,
        name: &str,
        ambiguous_types: &mut HashSet<TypeIdent>,
        ambiguous_functions: &mut HashSet<(String, RustFunctionId)>,
        ambiguous_constants: &mut HashSet<Ident>,
    ) {
        for (idx, ty) in self.types.iter().enumerate() {
            if ty.namespace.as_ref() != namespace {
                continue;
            }
            if ty.name.as_str() != name {
                continue;
            }
            insert_unambiguous(
                &mut self.by_imported_type,
                ambiguous_types,
                ty.name.clone(),
                idx,
            );
        }
        for (idx, function) in self.functions.iter().enumerate() {
            if function.namespace.as_ref() != namespace {
                continue;
            }
            let signature = &function.decl.item.signature;
            if signature.identifier.as_str() != name {
                continue;
            }
            let labels = signature.overload_labels();
            let labels = labels
                .iter()
                .map(|label| label.as_str())
                .collect::<Vec<_>>();
            let receiver = signature
                .receiver()
                .and_then(|param| match &param.param_type {
                    TypeElement::Plain(plain) => Some(&plain.ident),
                    TypeElement::Parametric(parametric) => Some(&parametric.base_type),
                    _ => None,
                });
            let id = RustFunctionId::new(receiver, signature.identifier.as_str(), &labels);
            insert_unambiguous(
                &mut self.by_imported_function,
                ambiguous_functions,
                ("".to_string(), id),
                idx,
            );
        }
        for (idx, constant) in self.constants.iter().enumerate() {
            if constant.namespace.as_ref() != namespace || constant.associated_receiver.is_some() {
                continue;
            }
            if constant.name.as_str() != name {
                continue;
            }
            insert_unambiguous(
                &mut self.by_imported_constant,
                ambiguous_constants,
                constant.name.clone(),
                idx,
            );
        }
    }
}

fn insert_unambiguous<K>(
    map: &mut HashMap<K, usize>,
    ambiguous: &mut HashSet<K>,
    key: K,
    idx: usize,
) where
    K: Clone + Eq + Hash,
{
    if ambiguous.contains(&key) {
        return;
    }

    if let Some(existing) = map.insert(key.clone(), idx) {
        if existing != idx {
            map.remove(&key);
            ambiguous.insert(key);
        }
    }
}

pub(super) fn imported_crates(uses: &[ToplevelItem<UseDecl>]) -> HashSet<String> {
    uses.iter()
        .filter_map(|use_decl| use_decl.path.segments.first())
        .map(|segment| segment.as_str().to_string())
        .collect()
}
