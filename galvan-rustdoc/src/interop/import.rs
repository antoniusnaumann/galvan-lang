use std::collections::HashSet;

use galvan_ast::TypeIdent;
use serde_json::Value;

use super::rustdoc_json::{
    constant_inner, constant_type, function_is_unsafe, impl_constant_ids, impl_function_ids,
    is_public, item_ids, item_inner, public_type_name, receiver_type_ident, return_is_borrowed,
    signature_contains_unliftable_type, trait_constant_ids, trait_function_ids,
    type_contains_unliftable_type,
};
use super::rustdoc_path::{callable_rust_path, impl_constant_rust_path, impl_function_rust_path};
use super::RustInterop;

impl RustInterop {
    pub fn add_crate(&mut self, crate_name: &str, json: &Value) {
        let Some(index) = json.get("index").and_then(Value::as_object) else {
            return;
        };

        let mut type_item_ids = Vec::new();
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            if public_type_name(item).is_some() {
                type_item_ids.push(item);
            }
        }

        for item in type_item_ids {
            self.push_type_from_item(crate_name, item, index);
        }

        let impl_function_ids = impl_function_ids(index);
        let impl_constant_ids = impl_constant_ids(index);
        let trait_function_ids = trait_function_ids(index);
        let trait_constant_ids = trait_constant_ids(index);
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            if item
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| impl_function_ids.contains(id))
            {
                continue;
            }
            if item
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| trait_function_ids.contains(id))
            {
                continue;
            }
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(function) = item_inner(item, "function") else {
                continue;
            };
            if function_is_unsafe(function) {
                continue;
            }
            let Some(signature) = function.get("sig") else {
                continue;
            };
            if signature_contains_unliftable_type(signature) {
                continue;
            }
            let rust_path = callable_rust_path(crate_name, name, item);
            let Some(imported) = self.function_decl(crate_name, name, signature) else {
                continue;
            };
            let borrowed_return = return_is_borrowed(signature);
            self.push_function(
                crate_name,
                name,
                rust_path,
                imported.decl,
                borrowed_return,
                imported.return_conversion,
                imported.arg_conversions,
            );
        }
        self.import_top_level_constants(crate_name, index, &impl_constant_ids, &trait_constant_ids);
        self.import_impl_functions(crate_name, index);
        self.import_trait_items(crate_name, index);
        self.import_public_reexports(crate_name, index);
    }

    fn import_impl_functions(&mut self, crate_name: &str, index: &serde_json::Map<String, Value>) {
        for impl_item in index.values() {
            let Some(impl_inner) = item_inner(impl_item, "impl") else {
                continue;
            };
            let Some(associated_receiver) = self.impl_associated_receiver(crate_name, impl_inner)
            else {
                continue;
            };
            self.import_impl_constants(crate_name, impl_inner, index, &associated_receiver);

            for id in item_ids(impl_inner, "items") {
                let Some(item) = index.get(id) else {
                    continue;
                };
                if !is_public(item) {
                    continue;
                }
                let Some(name) = item.get("name").and_then(Value::as_str) else {
                    continue;
                };
                let Some(function) = item_inner(item, "function") else {
                    continue;
                };
                if function_is_unsafe(function) {
                    continue;
                }
                let Some(signature) = function.get("sig") else {
                    continue;
                };
                if signature_contains_unliftable_type(signature) {
                    continue;
                }

                let Some(imported) =
                    self.impl_function_decl(crate_name, name, signature, impl_inner)
                else {
                    continue;
                };
                let rust_path = impl_function_rust_path(crate_name, name, item, impl_inner);
                let borrowed_return = return_is_borrowed(signature);
                self.push_function_with_associated_receiver(
                    crate_name,
                    name,
                    rust_path,
                    imported.decl,
                    borrowed_return,
                    Some(associated_receiver.clone()),
                    imported.return_conversion,
                    imported.arg_conversions,
                );
            }
        }
    }

    fn import_trait_items(&mut self, crate_name: &str, index: &serde_json::Map<String, Value>) {
        for trait_item in index.values() {
            if !is_public(trait_item) {
                continue;
            }
            let Some(name) = trait_item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(trait_inner) = item_inner(trait_item, "trait") else {
                continue;
            };
            let receiver = TypeIdent::new(name);

            for id in item_ids(trait_inner, "items") {
                let Some(item) = index.get(id) else {
                    continue;
                };
                if !is_public(item) {
                    continue;
                }
                let Some(item_name) = item.get("name").and_then(Value::as_str) else {
                    continue;
                };

                if let Some(function) = item_inner(item, "function") {
                    if function_is_unsafe(function) {
                        continue;
                    }
                    let Some(signature) = function.get("sig") else {
                        continue;
                    };
                    if signature_contains_unliftable_type(signature) {
                        continue;
                    }
                    let Some(imported) =
                        self.trait_function_decl(crate_name, item_name, signature, &receiver)
                    else {
                        continue;
                    };
                    let rust_path = callable_rust_path(crate_name, item_name, item);
                    let borrowed_return = return_is_borrowed(signature);
                    self.push_function_with_associated_receiver(
                        crate_name,
                        item_name,
                        rust_path,
                        imported.decl,
                        borrowed_return,
                        Some(receiver.clone()),
                        imported.return_conversion,
                        imported.arg_conversions,
                    );
                    continue;
                }

                if let Some(constant) = constant_inner(item) {
                    self.import_trait_constant(
                        crate_name,
                        item_name,
                        callable_rust_path(crate_name, item_name, item),
                        constant,
                        &receiver,
                    );
                }
            }
        }
    }

    fn import_public_reexports(&mut self, crate_name: &str, index: &serde_json::Map<String, Value>) {
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            let Some(use_item) = item_inner(item, "use") else {
                continue;
            };
            if use_item.get("is_glob").and_then(Value::as_bool) == Some(true) {
                self.import_glob_reexport(crate_name, item, use_item, index);
                continue;
            }
            let Some(exported_name) = item
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| use_item.get("name").and_then(Value::as_str))
            else {
                continue;
            };
            let Some(target_id) = use_item.get("id").and_then(Value::as_str) else {
                self.import_external_reexported_type(crate_name, exported_name, use_item);
                continue;
            };
            let Some(target) = index.get(target_id) else {
                self.import_external_reexported_type(crate_name, exported_name, use_item);
                continue;
            };
            let rust_path = callable_rust_path(crate_name, exported_name, item);
            self.import_reexport_target(crate_name, exported_name, rust_path, target, index);
        }
    }

    fn import_reexport_target(
        &mut self,
        crate_name: &str,
        exported_name: &str,
        rust_path: Box<str>,
        target: &Value,
        index: &serde_json::Map<String, Value>,
    ) {
        if public_type_name(target).is_some() {
            self.push_reexported_type_from_item(crate_name, exported_name, rust_path, target, index);
            return;
        }

        if let Some(function) = item_inner(target, "function") {
            if function_is_unsafe(function) {
                return;
            }
            let Some(signature) = function.get("sig") else {
                return;
            };
            if signature_contains_unliftable_type(signature) {
                return;
            }
            let Some(imported) = self.function_decl(crate_name, exported_name, signature) else {
                return;
            };
            let borrowed_return = return_is_borrowed(signature);
            self.push_function(
                crate_name,
                exported_name,
                rust_path,
                imported.decl,
                borrowed_return,
                imported.return_conversion,
                imported.arg_conversions,
            );
            return;
        }

        if let Some(constant) = constant_inner(target) {
            self.import_reexported_constant(crate_name, exported_name, rust_path, constant);
        }
    }

    fn import_external_reexported_type(
        &mut self,
        crate_name: &str,
        exported_name: &str,
        use_item: &Value,
    ) {
        if !looks_like_type_name(exported_name) {
            return;
        }

        let Some(source) = use_item.get("source").and_then(Value::as_str) else {
            return;
        };

        self.push_external_reexported_type(crate_name, exported_name, absolute_rust_path(source));
    }

    fn import_glob_reexport(
        &mut self,
        crate_name: &str,
        item: &Value,
        use_item: &Value,
        index: &serde_json::Map<String, Value>,
    ) {
        let Some(target_id) = use_item.get("id").and_then(Value::as_str) else {
            return;
        };
        let Some(module) = index
            .get(target_id)
            .and_then(|target| item_inner(target, "module"))
        else {
            return;
        };

        for id in item_ids(module, "items") {
            let Some(target) = index.get(id) else {
                continue;
            };
            if !is_public(target) {
                continue;
            }
            let Some(exported_name) = target.get("name").and_then(Value::as_str) else {
                continue;
            };
            let rust_path = callable_rust_path(crate_name, exported_name, item);
            self.import_reexport_target(crate_name, exported_name, rust_path, target, index);
        }
    }

    fn import_reexported_constant(
        &mut self,
        crate_name: &str,
        exported_name: &str,
        rust_path: Box<str>,
        constant: &Value,
    ) {
        let Some(constant_ty) = constant_type(constant) else {
            return;
        };
        if type_contains_unliftable_type(constant_ty) {
            return;
        }
        let Some(ty) = self.type_from_json(crate_name, constant_ty) else {
            return;
        };
        self.push_constant(crate_name, None, exported_name, rust_path, ty);
    }

    fn import_trait_constant(
        &mut self,
        crate_name: &str,
        name: &str,
        rust_path: Box<str>,
        constant: &Value,
        receiver: &TypeIdent,
    ) {
        let Some(constant_ty) = constant_type(constant) else {
            return;
        };
        if type_contains_unliftable_type(constant_ty) {
            return;
        }
        let Some(ty) = self.type_from_json(crate_name, constant_ty) else {
            return;
        };
        self.push_constant(crate_name, Some(receiver.clone()), name, rust_path, ty);
    }

    fn import_top_level_constants(
        &mut self,
        crate_name: &str,
        index: &serde_json::Map<String, Value>,
        impl_constant_ids: &HashSet<&str>,
        trait_constant_ids: &HashSet<&str>,
    ) {
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            if item
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| impl_constant_ids.contains(id))
            {
                continue;
            }
            if item
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| trait_constant_ids.contains(id))
            {
                continue;
            }
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(constant) = constant_inner(item) else {
                continue;
            };
            let Some(constant_ty) = constant_type(constant) else {
                continue;
            };
            if type_contains_unliftable_type(constant_ty) {
                continue;
            }
            let Some(ty) = self.type_from_json(crate_name, constant_ty) else {
                continue;
            };
            self.push_constant(
                crate_name,
                None,
                name,
                callable_rust_path(crate_name, name, item),
                ty,
            );
        }
    }

    fn import_impl_constants(
        &mut self,
        crate_name: &str,
        impl_inner: &Value,
        index: &serde_json::Map<String, Value>,
        receiver: &TypeIdent,
    ) {
        for id in item_ids(impl_inner, "items") {
            let Some(item) = index.get(id) else {
                continue;
            };
            if !is_public(item) {
                continue;
            }
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(constant) = constant_inner(item) else {
                continue;
            };
            let Some(constant_ty) = constant_type(constant) else {
                continue;
            };
            if type_contains_unliftable_type(constant_ty) {
                continue;
            }
            let Some(ty) = self.type_from_json(crate_name, constant_ty) else {
                continue;
            };
            let rust_path = impl_constant_rust_path(crate_name, name, item, impl_inner);
            self.push_constant(crate_name, Some(receiver.clone()), name, rust_path, ty);
        }
    }

    fn impl_associated_receiver(
        &mut self,
        crate_name: &str,
        impl_inner: &Value,
    ) -> Option<TypeIdent> {
        impl_inner
            .get("for")
            .and_then(|ty| self.type_from_json(crate_name, ty))
            .and_then(|ty| receiver_type_ident(&ty))
    }
}

fn absolute_rust_path(source: &str) -> Box<str> {
    if source.starts_with("::") {
        source.into()
    } else {
        format!("::{source}").into()
    }
}

fn looks_like_type_name(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase) && name.chars().any(char::is_lowercase)
}
