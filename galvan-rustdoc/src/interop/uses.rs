use std::collections::HashSet;

use galvan_ast::{ToplevelItem, TypeElement, UseDecl};

use super::function_id::RustFunctionId;
use super::RustInterop;

impl RustInterop {
    pub fn import_uses(&mut self, uses: &[ToplevelItem<UseDecl>]) {
        for use_decl in uses {
            let Some(namespace) = use_decl.path.segments.first() else {
                continue;
            };
            let namespace = namespace.as_str();
            match use_decl.path.segments.as_slice() {
                [_] => self.import_namespace(namespace),
                [_, item] => self.import_item(namespace, item.as_str()),
                _ => {}
            }
        }
    }

    fn import_namespace(&mut self, namespace: &str) {
        for (idx, ty) in self.types.iter().enumerate() {
            if ty.namespace.as_ref() != namespace {
                continue;
            }
            self.by_imported_type.insert(ty.name.clone(), idx);
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
            self.by_imported_function.insert(("".to_string(), id), idx);
        }
        for (idx, constant) in self.constants.iter().enumerate() {
            if constant.namespace.as_ref() != namespace || constant.associated_receiver.is_some() {
                continue;
            }
            self.by_imported_constant.insert(constant.name.clone(), idx);
        }
    }

    fn import_item(&mut self, namespace: &str, name: &str) {
        for (idx, ty) in self.types.iter().enumerate() {
            if ty.namespace.as_ref() != namespace {
                continue;
            }
            if ty.name.as_str() != name {
                continue;
            }
            self.by_imported_type.insert(ty.name.clone(), idx);
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
            self.by_imported_function.insert(("".to_string(), id), idx);
        }
        for (idx, constant) in self.constants.iter().enumerate() {
            if constant.namespace.as_ref() != namespace || constant.associated_receiver.is_some() {
                continue;
            }
            if constant.name.as_str() != name {
                continue;
            }
            self.by_imported_constant.insert(constant.name.clone(), idx);
        }
    }
}

pub(super) fn imported_crates(uses: &[ToplevelItem<UseDecl>]) -> HashSet<String> {
    uses.iter()
        .filter_map(|use_decl| use_decl.path.segments.first())
        .map(|segment| segment.as_str().to_string())
        .collect()
}
