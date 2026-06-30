use std::collections::HashSet;

use serde_json::Value;

use super::rustdoc_json::{
    callable_rust_path, constant_inner, constant_type, impl_constant_ids, impl_constant_rust_path,
    impl_function_ids, impl_function_rust_path, is_public, item_ids, item_inner, public_type_name,
    receiver_type_ident, return_is_borrowed,
};
use super::RustInterop;

impl RustInterop {
    pub fn add_crate(&mut self, crate_name: &str, json: &Value) {
        let Some(index) = json.get("index").and_then(Value::as_object) else {
            self.add_curated_crate(crate_name);
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
        let mut found_function = false;
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
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(function) = item_inner(item, "function") else {
                continue;
            };
            let Some(signature) = function.get("sig") else {
                continue;
            };
            let rust_path = callable_rust_path(crate_name, name, item);
            let imported = self.function_decl(crate_name, name, signature);
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
            found_function = true;
        }
        self.import_top_level_constants(crate_name, index, &impl_constant_ids);
        found_function |= self.import_impl_functions(crate_name, index);
        found_function |= self.import_public_reexports(crate_name, index);

        if !found_function {
            self.add_curated_crate(crate_name);
        }
    }

    fn import_impl_functions(
        &mut self,
        crate_name: &str,
        index: &serde_json::Map<String, Value>,
    ) -> bool {
        let mut found_function = false;
        for impl_item in index.values() {
            let Some(impl_inner) = item_inner(impl_item, "impl") else {
                continue;
            };
            self.import_impl_constants(crate_name, impl_inner, index);

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
                let Some(signature) = function.get("sig") else {
                    continue;
                };

                let associated_receiver = impl_inner
                    .get("for")
                    .and_then(|ty| self.type_from_json(crate_name, ty))
                    .and_then(|ty| receiver_type_ident(&ty));
                let imported = self.impl_function_decl(crate_name, name, signature, impl_inner);
                let rust_path = impl_function_rust_path(crate_name, name, item, impl_inner);
                let borrowed_return = return_is_borrowed(signature);
                self.push_function_with_associated_receiver(
                    crate_name,
                    name,
                    rust_path,
                    imported.decl,
                    borrowed_return,
                    associated_receiver,
                    imported.return_conversion,
                    imported.arg_conversions,
                );
                found_function = true;
            }
        }

        found_function
    }

    fn import_public_reexports(
        &mut self,
        crate_name: &str,
        index: &serde_json::Map<String, Value>,
    ) -> bool {
        let mut found_function = false;
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            let Some(use_item) = item_inner(item, "use") else {
                continue;
            };
            if use_item.get("is_glob").and_then(Value::as_bool) == Some(true) {
                found_function |= self.import_glob_reexport(crate_name, item, use_item, index);
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

            if public_type_name(target).is_some() {
                self.push_reexported_type_from_item(
                    crate_name,
                    exported_name,
                    rust_path,
                    target,
                    index,
                );
                continue;
            }

            if let Some(function) = item_inner(target, "function") {
                if let Some(signature) = function.get("sig") {
                    let imported = self.function_decl(crate_name, exported_name, signature);
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
                    found_function = true;
                }
                continue;
            }

            if let Some(constant) = constant_inner(target) {
                let Some(ty) =
                    constant_type(constant).and_then(|ty| self.type_from_json(crate_name, ty))
                else {
                    continue;
                };
                self.push_constant(crate_name, None, exported_name, rust_path, ty);
            }
        }
        found_function
    }

    fn import_external_reexported_type(
        &mut self,
        crate_name: &str,
        exported_name: &str,
        use_item: &Value,
    ) -> bool {
        if !looks_like_type_name(exported_name) {
            return false;
        }

        let Some(source) = use_item.get("source").and_then(Value::as_str) else {
            return false;
        };

        self.push_external_reexported_type(crate_name, exported_name, absolute_rust_path(source));
        true
    }

    fn import_glob_reexport(
        &mut self,
        crate_name: &str,
        item: &Value,
        use_item: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> bool {
        let Some(target_id) = use_item.get("id").and_then(Value::as_str) else {
            return false;
        };
        let Some(module) = index
            .get(target_id)
            .and_then(|target| item_inner(target, "module"))
        else {
            return false;
        };

        let mut found_function = false;
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

            if public_type_name(target).is_some() {
                self.push_reexported_type_from_item(
                    crate_name,
                    exported_name,
                    rust_path,
                    target,
                    index,
                );
                continue;
            }

            if let Some(function) = item_inner(target, "function") {
                if let Some(signature) = function.get("sig") {
                    let imported = self.function_decl(crate_name, exported_name, signature);
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
                    found_function = true;
                }
                continue;
            }

            if let Some(constant) = constant_inner(target) {
                let Some(ty) =
                    constant_type(constant).and_then(|ty| self.type_from_json(crate_name, ty))
                else {
                    continue;
                };
                self.push_constant(crate_name, None, exported_name, rust_path, ty);
            }
        }

        found_function
    }

    fn import_top_level_constants(
        &mut self,
        crate_name: &str,
        index: &serde_json::Map<String, Value>,
        impl_constant_ids: &HashSet<&str>,
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
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(constant) = constant_inner(item) else {
                continue;
            };
            let Some(ty) =
                constant_type(constant).and_then(|ty| self.type_from_json(crate_name, ty))
            else {
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
    ) {
        let receiver = impl_inner
            .get("for")
            .and_then(|ty| self.type_from_json(crate_name, ty))
            .and_then(|ty| receiver_type_ident(&ty));

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
            let Some(ty) =
                constant_type(constant).and_then(|ty| self.type_from_json(crate_name, ty))
            else {
                continue;
            };
            let rust_path = impl_constant_rust_path(crate_name, name, item, impl_inner);
            self.push_constant(crate_name, receiver.clone(), name, rust_path, ty);
        }
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
