use std::collections::HashSet;

use galvan_ast::TypeIdent;
use rustdoc_types::{Crate, Id, Impl, Item, ItemEnum, Use};

use super::rustdoc_json::{
    constant_type, function_is_unsafe, impl_constant_ids, impl_function_ids, is_public,
    public_type_name, receiver_type_ident, return_is_borrowed, signature_contains_unliftable_type,
    trait_constant_ids, trait_function_ids, type_contains_unliftable_type,
};
use super::rustdoc_path::{callable_rust_path, impl_constant_rust_path, impl_function_rust_path};
use super::RustInterop;

impl RustInterop {
    pub fn add_crate(&mut self, crate_name: &str, krate: &Crate) {
        // rustdoc's `index` is an unordered `HashMap`; walk it in a stable id
        // order so lifting (and any duplicate rust_path resolution) is
        // deterministic across runs and snapshot tests.
        let items = sorted_items(krate);

        for (_, item) in &items {
            if is_public(item) && public_type_name(item).is_some() {
                self.push_type_from_item(krate, crate_name, item);
            }
        }

        let impl_function_ids = impl_function_ids(krate);
        let impl_constant_ids = impl_constant_ids(krate);
        let trait_function_ids = trait_function_ids(krate);
        let trait_constant_ids = trait_constant_ids(krate);
        for (id, item) in &items {
            if !is_public(item) {
                continue;
            }
            if impl_function_ids.contains(id) || trait_function_ids.contains(id) {
                continue;
            }
            let Some(name) = item.name.as_deref() else {
                continue;
            };
            let ItemEnum::Function(function) = &item.inner else {
                continue;
            };
            if function_is_unsafe(function) {
                continue;
            }
            if signature_contains_unliftable_type(krate, &function.sig) {
                continue;
            }
            let rust_path = callable_rust_path(krate, crate_name, name, item);
            let Some(imported) = self.function_decl(krate, crate_name, name, &function.sig) else {
                continue;
            };
            let borrowed_return = return_is_borrowed(&function.sig);
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
        self.import_top_level_constants(krate, crate_name, &impl_constant_ids, &trait_constant_ids);
        self.import_impl_functions(krate, crate_name);
        self.import_trait_items(krate, crate_name);
        self.import_public_reexports(krate, crate_name);
    }

    fn import_impl_functions(&mut self, krate: &Crate, crate_name: &str) {
        for (_, impl_item) in sorted_items(krate) {
            let ItemEnum::Impl(impl_) = &impl_item.inner else {
                continue;
            };
            let Some(associated_receiver) = self.impl_associated_receiver(krate, crate_name, impl_)
            else {
                continue;
            };
            self.import_impl_constants(krate, crate_name, impl_, &associated_receiver);

            for id in &impl_.items {
                let Some(item) = krate.index.get(id) else {
                    continue;
                };
                if !is_public(item) {
                    continue;
                }
                let Some(name) = item.name.as_deref() else {
                    continue;
                };
                let ItemEnum::Function(function) = &item.inner else {
                    continue;
                };
                if function_is_unsafe(function) {
                    continue;
                }
                if signature_contains_unliftable_type(krate, &function.sig) {
                    continue;
                }

                let Some(imported) =
                    self.impl_function_decl(krate, crate_name, name, &function.sig, impl_)
                else {
                    continue;
                };
                let rust_path = impl_function_rust_path(krate, crate_name, name, item, impl_);
                let borrowed_return = return_is_borrowed(&function.sig);
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

    fn import_trait_items(&mut self, krate: &Crate, crate_name: &str) {
        for (_, trait_item) in sorted_items(krate) {
            if !is_public(trait_item) {
                continue;
            }
            let Some(name) = trait_item.name.as_deref() else {
                continue;
            };
            let ItemEnum::Trait(trait_) = &trait_item.inner else {
                continue;
            };
            let receiver = TypeIdent::new(name);

            for id in &trait_.items {
                let Some(item) = krate.index.get(id) else {
                    continue;
                };
                if !is_public(item) {
                    continue;
                }
                let Some(item_name) = item.name.as_deref() else {
                    continue;
                };

                if let ItemEnum::Function(function) = &item.inner {
                    if function_is_unsafe(function) {
                        continue;
                    }
                    if signature_contains_unliftable_type(krate, &function.sig) {
                        continue;
                    }
                    let Some(imported) = self.trait_function_decl(
                        krate,
                        crate_name,
                        item_name,
                        &function.sig,
                        &receiver,
                    ) else {
                        continue;
                    };
                    let rust_path = callable_rust_path(krate, crate_name, item_name, item);
                    let borrowed_return = return_is_borrowed(&function.sig);
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

                if constant_type(item).is_some() {
                    let rust_path = callable_rust_path(krate, crate_name, item_name, item);
                    self.import_trait_constant(krate, crate_name, item_name, rust_path, item, &receiver);
                }
            }
        }
    }

    fn import_public_reexports(&mut self, krate: &Crate, crate_name: &str) {
        for (_, item) in sorted_items(krate) {
            if !is_public(item) {
                continue;
            }
            let ItemEnum::Use(use_) = &item.inner else {
                continue;
            };
            if use_.is_glob {
                self.import_glob_reexport(krate, crate_name, item, use_);
                continue;
            }
            let exported_name = if item.name.as_deref().is_some_and(|name| !name.is_empty()) {
                item.name.as_deref().unwrap()
            } else {
                use_.name.as_str()
            };
            let Some(target_id) = use_.id else {
                self.import_external_reexported_type(crate_name, exported_name, use_);
                continue;
            };
            let Some(target) = krate.index.get(&target_id) else {
                self.import_external_reexported_type(crate_name, exported_name, use_);
                continue;
            };
            let rust_path = callable_rust_path(krate, crate_name, exported_name, item);
            self.import_reexport_target(krate, crate_name, exported_name, rust_path, target);
        }
    }

    fn import_reexport_target(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        exported_name: &str,
        rust_path: Box<str>,
        target: &Item,
    ) {
        if public_type_name(target).is_some() {
            self.push_reexported_type_from_item(krate, crate_name, exported_name, rust_path, target);
            return;
        }

        if let ItemEnum::Function(function) = &target.inner {
            if function_is_unsafe(function) {
                return;
            }
            if signature_contains_unliftable_type(krate, &function.sig) {
                return;
            }
            let Some(imported) = self.function_decl(krate, crate_name, exported_name, &function.sig)
            else {
                return;
            };
            let borrowed_return = return_is_borrowed(&function.sig);
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

        if constant_type(target).is_some() {
            self.import_reexported_constant(krate, crate_name, exported_name, rust_path, target);
        }
    }

    fn import_external_reexported_type(&mut self, crate_name: &str, exported_name: &str, use_: &Use) {
        if !looks_like_type_name(exported_name) {
            return;
        }

        self.push_external_reexported_type(
            crate_name,
            exported_name,
            absolute_rust_path(&use_.source),
        );
    }

    fn import_glob_reexport(&mut self, krate: &Crate, crate_name: &str, item: &Item, use_: &Use) {
        let Some(target_id) = use_.id else {
            return;
        };
        let Some(ItemEnum::Module(module)) = krate.index.get(&target_id).map(|target| &target.inner)
        else {
            return;
        };

        for id in &module.items {
            let Some(target) = krate.index.get(id) else {
                continue;
            };
            if !is_public(target) {
                continue;
            }
            let Some(exported_name) = target.name.as_deref() else {
                continue;
            };
            let rust_path = callable_rust_path(krate, crate_name, exported_name, item);
            self.import_reexport_target(krate, crate_name, exported_name, rust_path, target);
        }
    }

    fn import_reexported_constant(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        exported_name: &str,
        rust_path: Box<str>,
        constant: &Item,
    ) {
        let Some(constant_ty) = constant_type(constant) else {
            return;
        };
        if type_contains_unliftable_type(krate, constant_ty) {
            return;
        }
        let Some(ty) = self.type_from_json(krate, crate_name, constant_ty) else {
            return;
        };
        self.push_constant(crate_name, None, exported_name, rust_path, ty);
    }

    fn import_trait_constant(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        rust_path: Box<str>,
        constant: &Item,
        receiver: &TypeIdent,
    ) {
        let Some(constant_ty) = constant_type(constant) else {
            return;
        };
        if type_contains_unliftable_type(krate, constant_ty) {
            return;
        }
        let Some(ty) = self.type_from_json(krate, crate_name, constant_ty) else {
            return;
        };
        self.push_constant(crate_name, Some(receiver.clone()), name, rust_path, ty);
    }

    fn import_top_level_constants(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        impl_constant_ids: &HashSet<Id>,
        trait_constant_ids: &HashSet<Id>,
    ) {
        for (id, item) in sorted_items(krate) {
            if !is_public(item) {
                continue;
            }
            if impl_constant_ids.contains(id) || trait_constant_ids.contains(id) {
                continue;
            }
            let Some(name) = item.name.as_deref() else {
                continue;
            };
            let Some(constant_ty) = constant_type(item) else {
                continue;
            };
            if type_contains_unliftable_type(krate, constant_ty) {
                continue;
            }
            let Some(ty) = self.type_from_json(krate, crate_name, constant_ty) else {
                continue;
            };
            self.push_constant(
                crate_name,
                None,
                name,
                callable_rust_path(krate, crate_name, name, item),
                ty,
            );
        }
    }

    fn import_impl_constants(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        impl_: &Impl,
        receiver: &TypeIdent,
    ) {
        for id in &impl_.items {
            let Some(item) = krate.index.get(id) else {
                continue;
            };
            if !is_public(item) {
                continue;
            }
            let Some(name) = item.name.as_deref() else {
                continue;
            };
            let Some(constant_ty) = constant_type(item) else {
                continue;
            };
            if type_contains_unliftable_type(krate, constant_ty) {
                continue;
            }
            let Some(ty) = self.type_from_json(krate, crate_name, constant_ty) else {
                continue;
            };
            let rust_path = impl_constant_rust_path(krate, crate_name, name, item, impl_);
            self.push_constant(crate_name, Some(receiver.clone()), name, rust_path, ty);
        }
    }

    fn impl_associated_receiver(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        impl_: &Impl,
    ) -> Option<TypeIdent> {
        self.type_from_json(krate, crate_name, &impl_.for_)
            .and_then(|ty| receiver_type_ident(&ty))
    }
}

/// The items of a crate's `index`, ordered by their stable rustdoc `Id` so
/// iteration is deterministic (the underlying map is a `HashMap`).
fn sorted_items(krate: &Crate) -> Vec<(&Id, &Item)> {
    let mut items: Vec<_> = krate.index.iter().collect();
    items.sort_by_key(|(id, _)| **id);
    items
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
