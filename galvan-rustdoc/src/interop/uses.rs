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
                    None,
                    &mut ambiguous_types,
                    &mut ambiguous_functions,
                    &mut ambiguous_constants,
                ),
                segments => {
                    let rust_path = format!(
                        "::{}",
                        segments
                            .iter()
                            .map(|segment| segment.as_str())
                            .collect::<Vec<_>>()
                            .join("::")
                    );
                    self.import_namespace(
                        namespace,
                        Some(&rust_path),
                        &mut ambiguous_types,
                        &mut ambiguous_functions,
                        &mut ambiguous_constants,
                    )
                }
            }
        }
    }

    fn import_namespace(
        &mut self,
        namespace: &str,
        rust_path: Option<&str>,
        ambiguous_types: &mut HashSet<TypeIdent>,
        ambiguous_functions: &mut HashSet<RustFunctionId>,
        ambiguous_constants: &mut HashSet<Ident>,
    ) {
        // Collapse duplicates within this namespace before the cross-namespace
        // ambiguity check: an item that is both defined and `pub use`-re-exported
        // (e.g. `serde_json::ser::to_string` plus a crate-root re-export) appears
        // several times in the registry under different `rust_path`s, but it is
        // one logical item and must not read as ambiguous against itself.
        let mut types: HashMap<TypeIdent, usize> = HashMap::new();
        for (idx, ty) in self.types.iter().enumerate() {
            if ty.namespace.as_ref() != namespace {
                continue;
            }
            if rust_path.is_some_and(|rust_path| ty.rust_path.as_ref() != rust_path) {
                continue;
            }
            types.insert(ty.name.clone(), idx);
        }
        for (key, idx) in types {
            insert_unambiguous(&mut self.by_imported_type, ambiguous_types, key, idx);
        }

        let mut functions: HashMap<RustFunctionId, usize> = HashMap::new();
        for (idx, function) in self.functions.iter().enumerate() {
            if function.namespace.as_ref() != namespace {
                continue;
            }
            let signature = &function.decl.item.signature;
            if rust_path.is_some_and(|rust_path| function.rust_path.as_ref() != rust_path) {
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
            functions.insert(id, idx);
        }
        for (key, idx) in functions {
            insert_unambiguous(
                &mut self.by_imported_function,
                ambiguous_functions,
                key,
                idx,
            );
        }

        let mut constants: HashMap<Ident, usize> = HashMap::new();
        for (idx, constant) in self.constants.iter().enumerate() {
            if constant.namespace.as_ref() != namespace || constant.associated_receiver.is_some() {
                continue;
            }
            if rust_path.is_some_and(|rust_path| constant.rust_path.as_ref() != rust_path) {
                continue;
            }
            constants.insert(constant.name.clone(), idx);
        }
        for (key, idx) in constants {
            insert_unambiguous(
                &mut self.by_imported_constant,
                ambiguous_constants,
                key,
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
