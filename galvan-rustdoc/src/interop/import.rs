use std::collections::HashSet;

use rustdoc_types::{Crate, Id, Impl, Item, ItemEnum, Use};

use galvan_ast::{FnSignature, TypeElement, TypeIdent};

use crate::model::RustdocCrateLiftSummary;

use super::rustdoc_json::{
    constant_type, function_is_unliftable, impl_constant_ids, impl_function_ids, is_public,
    public_type_name, receiver_type_ident, return_is_borrowed, trait_constant_ids,
    trait_function_ids, type_contains_unliftable_type,
};
use super::registry::item_source_span;
use super::rustdoc_path::{
    callable_rust_path, extension_trait_rust_path, impl_constant_rust_path, impl_function_rust_path,
};
use super::RustInterop;

struct ReexportedImplImport<'a> {
    krate: &'a Crate,
    crate_name: &'a str,
    impl_: &'a Impl,
    original_receiver: &'a TypeIdent,
    exported_receiver: &'a TypeIdent,
    receiver_rust_path: &'a str,
}

impl RustInterop {
    /// Resolve the relative source-span paths of `crate_name`'s lifted items
    /// against the crate's source directory. rustdoc emits span filenames
    /// relative to the documented crate, not the consumer project.
    pub fn resolve_source_spans(&mut self, crate_name: &str, dependency_root: &std::path::Path) {
        let resolve = |span: &mut Option<crate::model::RustSourceSpan>| {
            if let Some(span) = span {
                if span.path.is_relative() {
                    span.path = dependency_root.join(&span.path);
                }
            }
        };
        for ty in self.types.iter_mut().filter(|ty| ty.namespace.as_ref() == crate_name) {
            resolve(&mut ty.source_span);
        }
        for function in self
            .functions
            .iter_mut()
            .filter(|function| function.namespace.as_ref() == crate_name)
        {
            resolve(&mut function.source_span);
        }
        for constant in self
            .constants
            .iter_mut()
            .filter(|constant| constant.namespace.as_ref() == crate_name)
        {
            resolve(&mut constant.source_span);
        }
    }

    pub fn add_crate(&mut self, crate_name: &str, krate: &Crate) -> RustdocCrateLiftSummary {
        let before = LiftCounts::from(&*self);

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
            if function_is_unliftable(krate, function) {
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
                item_source_span(item),
            );
        }
        self.import_top_level_constants(krate, crate_name, &impl_constant_ids, &trait_constant_ids);
        self.import_impl_functions(krate, crate_name);
        self.import_trait_items(krate, crate_name);
        self.import_public_reexports(krate, crate_name);

        let summary = RustdocCrateLiftSummary {
            crate_name: crate_name.into(),
            types: self.types.len().saturating_sub(before.types),
            functions: self.functions.len().saturating_sub(before.functions),
            constants: self.constants.len().saturating_sub(before.constants),
        };
        self.lift_summaries
            .insert(summary.crate_name.clone(), summary.clone());
        summary
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
            let extension_trait = match &impl_.trait_ {
                Some(_) => {
                    let Some(extension_trait) =
                        extension_trait_rust_path(krate, crate_name, impl_, &associated_receiver)
                    else {
                        continue;
                    };
                    Some(extension_trait)
                }
                None => None,
            };

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
                if function_is_unliftable(krate, function) {
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
                    extension_trait.clone(),
                    imported.return_conversion,
                    imported.arg_conversions,
                    item_source_span(item),
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
                    if function_is_unliftable(krate, function) {
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
                        None,
                        imported.return_conversion,
                        imported.arg_conversions,
                        item_source_span(item),
                    );
                    continue;
                }

                if constant_type(item).is_some() {
                    let rust_path = callable_rust_path(krate, crate_name, item_name, item);
                    self.import_trait_constant(
                        krate, crate_name, item_name, rust_path, item, &receiver,
                    );
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
                // A primitive re-export (`pub use i32 as …`): rustdoc records no
                // target id, so the `use` source is the item's absolute path.
                self.import_external_reexported_type(crate_name, exported_name, use_);
                continue;
            };
            let Some(target) = krate.index.get(&target_id) else {
                // An intra-crate re-export of an item rustdoc did not include in
                // the index (e.g. a foreign type aliased through a private module,
                // such as serde_json's `pub use self::imp::Result`). Register it as
                // an opaque type under this re-export's own public path — not the
                // module-relative `use` source (`self::…`/`crate::…`), which is not
                // a valid absolute path — so it dedupes against any richer decl
                // already imported at that path.
                if looks_like_type_name(exported_name) {
                    let rust_path = callable_rust_path(krate, crate_name, exported_name, item);
                    self.push_external_reexported_type(crate_name, exported_name, rust_path);
                }
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
        if let Some(original_name) = public_type_name(target) {
            self.push_reexported_type_from_item(
                krate,
                crate_name,
                exported_name,
                rust_path.clone(),
                target,
            );
            self.import_reexported_impl_items(
                krate,
                crate_name,
                &TypeIdent::new(original_name),
                &TypeIdent::new(exported_name),
                rust_path,
            );
            return;
        }

        if let ItemEnum::Function(function) = &target.inner {
            if function_is_unliftable(krate, function) {
                return;
            }
            let Some(imported) =
                self.function_decl(krate, crate_name, exported_name, &function.sig)
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
                item_source_span(target),
            );
            return;
        }

        if constant_type(target).is_some() {
            self.import_reexported_constant(krate, crate_name, exported_name, rust_path, target);
        }
    }

    fn import_external_reexported_type(
        &mut self,
        crate_name: &str,
        exported_name: &str,
        use_: &Use,
    ) {
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
        let Some(ItemEnum::Module(module)) =
            krate.index.get(&target_id).map(|target| &target.inner)
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
        self.push_constant(
            crate_name,
            None,
            exported_name,
            rust_path,
            ty,
            item_source_span(constant),
        );
    }

    fn import_reexported_impl_items(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        original_receiver: &TypeIdent,
        exported_receiver: &TypeIdent,
        receiver_rust_path: Box<str>,
    ) {
        for (_, impl_item) in sorted_items(krate) {
            let ItemEnum::Impl(impl_) = &impl_item.inner else {
                continue;
            };
            if impl_.trait_.is_some() {
                continue;
            }
            let Some(receiver) = self.impl_associated_receiver(krate, crate_name, impl_) else {
                continue;
            };
            if &receiver != original_receiver {
                continue;
            }

            let import = ReexportedImplImport {
                krate,
                crate_name,
                impl_,
                original_receiver,
                exported_receiver,
                receiver_rust_path: receiver_rust_path.as_ref(),
            };
            self.import_reexported_impl_constants(&import);
            self.import_reexported_impl_functions(&import);
        }
    }

    fn import_reexported_impl_constants(&mut self, import: &ReexportedImplImport<'_>) {
        for id in &import.impl_.items {
            let Some(item) = import.krate.index.get(id) else {
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
            if type_contains_unliftable_type(import.krate, constant_ty) {
                continue;
            }
            let Some(mut ty) = self.type_from_json(import.krate, import.crate_name, constant_ty)
            else {
                continue;
            };
            rename_type_references(&mut ty, import.original_receiver, import.exported_receiver);
            self.push_constant(
                import.crate_name,
                Some(import.exported_receiver.clone()),
                name,
                format!("{}::{name}", import.receiver_rust_path).into_boxed_str(),
                ty,
                item_source_span(item),
            );
        }
    }

    fn import_reexported_impl_functions(&mut self, import: &ReexportedImplImport<'_>) {
        let extension_trait = match &import.impl_.trait_ {
            Some(_) => {
                let Some(extension_trait) = extension_trait_rust_path(
                    import.krate,
                    import.crate_name,
                    import.impl_,
                    import.original_receiver,
                ) else {
                    return;
                };
                Some(extension_trait)
            }
            None => None,
        };

        for id in &import.impl_.items {
            let Some(item) = import.krate.index.get(id) else {
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
            if function_is_unliftable(import.krate, function) {
                continue;
            }

            let Some(mut imported) = self.impl_function_decl(
                import.krate,
                import.crate_name,
                name,
                &function.sig,
                import.impl_,
            ) else {
                continue;
            };
            rename_function_type_references(
                &mut imported.decl.signature,
                import.original_receiver,
                import.exported_receiver,
            );
            let borrowed_return = return_is_borrowed(&function.sig);
            self.push_function_with_associated_receiver(
                import.crate_name,
                name,
                format!("{}::{name}", import.receiver_rust_path).into_boxed_str(),
                imported.decl,
                borrowed_return,
                Some(import.exported_receiver.clone()),
                extension_trait.clone(),
                imported.return_conversion,
                imported.arg_conversions,
                item_source_span(item),
            );
        }
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
        self.push_constant(
            crate_name,
            Some(receiver.clone()),
            name,
            rust_path,
            ty,
            item_source_span(constant),
        );
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
                item_source_span(item),
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
            self.push_constant(
                crate_name,
                Some(receiver.clone()),
                name,
                rust_path,
                ty,
                item_source_span(item),
            );
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

struct LiftCounts {
    types: usize,
    functions: usize,
    constants: usize,
}

impl From<&RustInterop> for LiftCounts {
    fn from(interop: &RustInterop) -> Self {
        Self {
            types: interop.types.len(),
            functions: interop.functions.len(),
            constants: interop.constants.len(),
        }
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

fn rename_function_type_references(
    signature: &mut FnSignature,
    original: &TypeIdent,
    exported: &TypeIdent,
) {
    for param in &mut signature.parameters.params {
        rename_type_references(&mut param.param_type, original, exported);
    }
    rename_type_references(&mut signature.return_type, original, exported);
}

fn rename_type_references(ty: &mut TypeElement, original: &TypeIdent, exported: &TypeIdent) {
    match ty {
        TypeElement::Plain(plain) => {
            if &plain.ident == original {
                plain.ident = exported.clone();
            }
        }
        TypeElement::Parametric(parametric) => {
            if &parametric.base_type == original {
                parametric.base_type = exported.clone();
            }
            for arg in &mut parametric.type_args {
                rename_type_references(arg, original, exported);
            }
        }
        TypeElement::Array(array) => {
            rename_type_references(&mut array.elements, original, exported)
        }
        TypeElement::Dictionary(dictionary) => {
            rename_type_references(&mut dictionary.key, original, exported);
            rename_type_references(&mut dictionary.value, original, exported);
        }
        TypeElement::OrderedDictionary(dictionary) => {
            rename_type_references(&mut dictionary.key, original, exported);
            rename_type_references(&mut dictionary.value, original, exported);
        }
        TypeElement::Set(set) => rename_type_references(&mut set.elements, original, exported),
        TypeElement::Tuple(tuple) => {
            for element in &mut tuple.elements {
                rename_type_references(element, original, exported);
            }
        }
        TypeElement::Optional(optional) => {
            rename_type_references(&mut optional.inner, original, exported);
        }
        TypeElement::Result(result) => {
            rename_type_references(&mut result.success, original, exported);
            if let Some(error) = &mut result.error {
                rename_type_references(error, original, exported);
            }
        }
        TypeElement::Closure(closure) => {
            for param in &mut closure.parameters {
                rename_type_references(param, original, exported);
            }
            rename_type_references(&mut closure.return_ty, original, exported);
        }
        TypeElement::Generic(_)
        | TypeElement::Infer(_)
        | TypeElement::Never(_)
        | TypeElement::Void(_) => {}
    }
}
