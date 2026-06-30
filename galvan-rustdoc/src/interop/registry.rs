use serde_json::Value;

use galvan_ast::{
    EmptyTypeDecl, FnDecl, Ident, Span, ToplevelItem, TypeDecl, TypeElement, TypeIdent, Visibility,
};
use galvan_files::Source;

use crate::model::{
    RustArgConversion, RustConstantDecl, RustFunctionDecl, RustReturnConversion, RustTypeDecl,
};

use super::function_id::RustFunctionId;
use super::lift_model::ImportedTypeDecl;
use super::rustdoc_json::{public_type_name, receiver_type_ident, rust_path};
use super::RustInterop;

impl RustInterop {
    pub(super) fn push_type(&mut self, crate_name: &str, name: &str) {
        let rust_path = format!("::{crate_name}::{name}").into_boxed_str();
        if self
            .types
            .iter()
            .any(|ty| ty.rust_path.as_ref() == rust_path.as_ref())
        {
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
                    span: Span::default(),
                }),
                source: Source::Builtin,
            },
        });
    }

    pub(super) fn push_type_from_item(
        &mut self,
        crate_name: &str,
        item: &Value,
        index: &serde_json::Map<String, Value>,
    ) {
        let Some(name) = public_type_name(item) else {
            return;
        };

        let rust_path = rust_path(crate_name, name, item);
        let imported = self
            .type_decl_from_item(crate_name, name, item, index)
            .unwrap_or_else(|| ImportedTypeDecl::empty(name));
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
        crate_name: &str,
        exported_name: &str,
        rust_path: Box<str>,
        item: &Value,
        index: &serde_json::Map<String, Value>,
    ) {
        if let Some(existing) = self
            .types
            .iter_mut()
            .find(|ty| ty.rust_path.as_ref() == rust_path.as_ref())
        {
            existing.name = TypeIdent::new(exported_name);
            return;
        }

        let imported = self
            .type_decl_from_item(crate_name, exported_name, item, index)
            .unwrap_or_else(|| ImportedTypeDecl::empty(exported_name));
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
        name: &str,
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
            name,
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
        if !has_receiver {
            if let Some(associated_receiver) = associated_receiver {
                self.by_namespace_associated_function.insert(
                    (crate_name.to_string(), associated_receiver, id.clone()),
                    idx,
                );
            } else {
                self.by_namespace_function
                    .insert((crate_name.to_string(), id.clone()), idx);
            }
        } else {
            self.by_namespace_function
                .insert((crate_name.to_string(), id.clone()), idx);
        }
    }
}
