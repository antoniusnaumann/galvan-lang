use rustdoc_types::{Crate, Item, Path};

use galvan_ast::{
    EmptyTypeDecl, FnDecl, Ident, Span, ToplevelItem, TypeDecl, TypeElement, TypeIdent, Visibility,
};
use galvan_files::Source;

use crate::model::{
    RustArgConversion, RustConstantDecl, RustFunctionDecl, RustReturnConversion, RustTypeDecl,
};

use super::function_id::RustFunctionId;
use super::lift_model::ImportedTypeDecl;
use super::rustdoc_json::{
    public_type_name, receiver_type_ident, resolved_type_generic_params, type_generic_params,
    type_has_unliftable_generics,
};
use super::rustdoc_path::{resolved_type_rust_path, rust_path};
use super::state::Unambiguous;
use super::RustInterop;

impl RustInterop {
    pub(super) fn push_resolved_type(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        resolved: &Path,
    ) {
        let rust_path = resolved_type_rust_path(krate, crate_name, name, resolved);
        self.push_empty_type(
            crate_name,
            name,
            rust_path,
            resolved_type_generic_params(resolved),
        );
    }

    fn push_empty_type(
        &mut self,
        crate_name: &str,
        name: &str,
        rust_path: Box<str>,
        generic_params: Vec<Ident>,
    ) {
        if let Some(existing) = self
            .types
            .iter()
            .position(|ty| ty.rust_path.as_ref() == rust_path.as_ref())
        {
            self.update_empty_type_generics(existing, generic_params);
            return;
        }

        let ident = TypeIdent::new(name);
        self.types.push(RustTypeDecl {
            namespace: crate_name.into(),
            name: ident.clone(),
            rust_path,
            field_conversions: Vec::new(),
            constructor_arg_conversions: Vec::new(),
            enum_variant_conversions: Vec::new(),
            decl: ToplevelItem {
                item: TypeDecl::Empty(EmptyTypeDecl {
                    visibility: Visibility::public(),
                    ident,
                    generic_params,
                    span: Span::default(),
                }),
                source: Source::Builtin,
            },
        });
    }

    fn update_empty_type_generics(&mut self, index: usize, generic_params: Vec<Ident>) {
        if generic_params.is_empty() {
            return;
        }

        let TypeDecl::Empty(empty) = &mut self.types[index].decl.item else {
            return;
        };
        if generic_params.len() > empty.generic_params.len() {
            empty.generic_params = generic_params;
        }
    }

    pub(super) fn push_type_from_item(&mut self, krate: &Crate, crate_name: &str, item: &Item) {
        if type_has_unliftable_generics(item) {
            return;
        }
        let Some(name) = public_type_name(item) else {
            return;
        };

        let rust_path = rust_path(krate, crate_name, name, item);
        let imported = self
            .type_decl_from_item(krate, crate_name, name, item)
            .unwrap_or_else(|| {
                ImportedTypeDecl::empty_with_generics(name, type_generic_params(item))
            });
        let type_decl = RustTypeDecl {
            namespace: crate_name.into(),
            name: TypeIdent::new(name),
            rust_path,
            field_conversions: imported.field_conversions,
            constructor_arg_conversions: imported.constructor_arg_conversions,
            enum_variant_conversions: imported.enum_variant_conversions,
            decl: ToplevelItem {
                item: imported.decl,
                source: Source::Builtin,
            },
        };

        if let Some(existing) = self
            .types
            .iter_mut()
            .find(|ty| ty.rust_path.as_ref() == type_decl.rust_path.as_ref())
        {
            if matches!(existing.decl.item, TypeDecl::Empty(_)) {
                *existing = type_decl;
            }
            return;
        }

        self.types.push(type_decl);
    }

    pub(super) fn push_reexported_type_from_item(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        exported_name: &str,
        rust_path: Box<str>,
        item: &Item,
    ) {
        if type_has_unliftable_generics(item) {
            return;
        }
        let imported = self
            .type_decl_from_item(krate, crate_name, exported_name, item)
            .unwrap_or_else(|| {
                ImportedTypeDecl::empty_with_generics(exported_name, type_generic_params(item))
            });

        if let Some(existing) = self
            .types
            .iter_mut()
            .find(|ty| ty.rust_path.as_ref() == rust_path.as_ref())
        {
            // Upgrade a placeholder opaque decl (e.g. an out-of-index re-export
            // registered at the same public path) to this richer lifted decl;
            // otherwise just adopt the re-export name.
            if matches!(existing.decl.item, TypeDecl::Empty(_))
                && !matches!(imported.decl, TypeDecl::Empty(_))
            {
                existing.field_conversions = imported.field_conversions;
                existing.constructor_arg_conversions = imported.constructor_arg_conversions;
                existing.enum_variant_conversions = imported.enum_variant_conversions;
                existing.decl.item = imported.decl;
            }
            existing.name = TypeIdent::new(exported_name);
            return;
        }

        self.types.push(RustTypeDecl {
            namespace: crate_name.into(),
            name: TypeIdent::new(exported_name),
            rust_path,
            field_conversions: imported.field_conversions,
            constructor_arg_conversions: imported.constructor_arg_conversions,
            enum_variant_conversions: imported.enum_variant_conversions,
            decl: ToplevelItem {
                item: imported.decl,
                source: Source::Builtin,
            },
        });
    }

    pub(super) fn push_external_reexported_type(
        &mut self,
        crate_name: &str,
        exported_name: &str,
        rust_path: Box<str>,
    ) {
        if let Some(existing) = self
            .types
            .iter_mut()
            .find(|ty| ty.rust_path.as_ref() == rust_path.as_ref())
        {
            existing.name = TypeIdent::new(exported_name);
            return;
        }

        let imported = ImportedTypeDecl::empty(exported_name);
        self.types.push(RustTypeDecl {
            namespace: crate_name.into(),
            name: TypeIdent::new(exported_name),
            rust_path,
            field_conversions: imported.field_conversions,
            constructor_arg_conversions: imported.constructor_arg_conversions,
            enum_variant_conversions: imported.enum_variant_conversions,
            decl: ToplevelItem {
                item: imported.decl,
                source: Source::Builtin,
            },
        });
    }

    pub(super) fn push_function(
        &mut self,
        crate_name: &str,
        name: &str,
        rust_path: Box<str>,
        decl: FnDecl,
        borrowed_return: bool,
        return_conversion: RustReturnConversion,
        arg_conversions: Vec<RustArgConversion>,
    ) {
        self.push_function_with_associated_receiver(
            crate_name,
            name,
            rust_path,
            decl,
            borrowed_return,
            None,
            return_conversion,
            arg_conversions,
        );
    }

    pub(super) fn push_constant(
        &mut self,
        crate_name: &str,
        associated_receiver: Option<TypeIdent>,
        name: &str,
        rust_path: Box<str>,
        ty: TypeElement,
    ) {
        let idx = self.constants.len();
        let ident = Ident::new(name);
        self.constants.push(RustConstantDecl {
            namespace: crate_name.into(),
            associated_receiver: associated_receiver.clone(),
            name: ident.clone(),
            rust_path,
            ty,
        });

        if let Some(receiver) = associated_receiver {
            insert_unambiguous_associated(
                &mut self.by_associated_constant,
                (receiver.clone(), ident.clone()),
                crate_name,
                idx,
            );
            self.by_namespace_associated_constant
                .insert((crate_name.to_string(), receiver, ident), idx);
        } else {
            self.by_namespace_constant
                .insert((crate_name.to_string(), ident), idx);
        }
    }

    pub(super) fn push_function_with_associated_receiver(
        &mut self,
        crate_name: &str,
        _name: &str,
        rust_path: Box<str>,
        decl: FnDecl,
        borrowed_return: bool,
        associated_receiver: Option<TypeIdent>,
        return_conversion: RustReturnConversion,
        arg_conversions: Vec<RustArgConversion>,
    ) {
        let labels = decl.signature.overload_labels();
        let labels = labels
            .iter()
            .map(|label| label.as_str())
            .collect::<Vec<_>>();
        let has_receiver = decl
            .signature
            .receiver()
            .and_then(|param| receiver_type_ident(&param.param_type))
            .is_some();
        let id = RustFunctionId::new(
            decl.signature
                .receiver()
                .and_then(|param| receiver_type_ident(&param.param_type))
                .as_ref(),
            decl.signature.identifier.as_str(),
            &labels,
        );
        let idx = self.functions.len();
        self.functions.push(RustFunctionDecl {
            namespace: crate_name.into(),
            rust_path,
            borrowed_return,
            return_conversion,
            arg_conversions,
            decl: ToplevelItem {
                item: decl,
                source: Source::Builtin,
            },
        });
        let has_associated_receiver = associated_receiver.is_some();
        if let Some(associated_receiver) = associated_receiver {
            insert_unambiguous_associated(
                &mut self.by_associated_function,
                (associated_receiver.clone(), id.clone()),
                crate_name,
                idx,
            );
            self.by_namespace_associated_function.insert(
                (crate_name.to_string(), associated_receiver, id.clone()),
                idx,
            );
        }
        if has_receiver || !has_associated_receiver {
            self.by_namespace_function
                .insert((crate_name.to_string(), id.clone()), idx);
        }
    }
}

/// Records that `namespace` exposes the associated `key`, tracking whether the
/// key remains exposed by a single namespace (`One`) or becomes ambiguous
/// (`Many`). Mirrors the overwrite semantics of the namespaced maps: a repeated
/// insert from the same namespace keeps the latest index and stays unambiguous.
fn insert_unambiguous_associated<K>(
    map: &mut std::collections::HashMap<K, Unambiguous>,
    key: K,
    namespace: &str,
    idx: usize,
) where
    K: Eq + std::hash::Hash,
{
    match map.get_mut(&key) {
        None => {
            map.insert(
                key,
                Unambiguous::One {
                    namespace: namespace.to_string(),
                    idx,
                },
            );
            return;
        }
        Some(Unambiguous::One {
            namespace: existing_namespace,
            idx: existing_idx,
        }) => {
            if existing_namespace == namespace {
                *existing_idx = idx;
                return;
            }
        }
        Some(Unambiguous::Many) => return,
    }
    map.insert(key, Unambiguous::Many);
}
