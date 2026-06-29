use std::collections::{HashMap, HashSet};
use std::fs;

use serde_json::Value;

use galvan_ast::{
    EmptyTypeDecl, FnDecl, Ident, Span, ToplevelItem, TypeDecl, TypeElement, TypeIdent, UseDecl,
    Visibility,
};
use galvan_files::Source;

use crate::cache::RustdocCache;
use crate::model::{
    RustArgConversion, RustConstantDecl, RustEnumVariantArgConversion, RustFunctionDecl,
    RustReturnConversion, RustTypeDecl,
};
use crate::RustdocError;

mod curated;
mod function_id;
mod import;
mod lift;
mod rustdoc_json;

use self::function_id::RustFunctionId;
use self::lift::ImportedTypeDecl;
use self::rustdoc_json::{public_type_name, receiver_type_ident, rust_path};

#[cfg(test)]
use self::lift::{generic_type, plain_type};

#[derive(Debug, Default)]
pub struct RustInterop {
    pub types: Vec<RustTypeDecl>,
    pub functions: Vec<RustFunctionDecl>,
    pub constants: Vec<RustConstantDecl>,
    by_imported_type: HashMap<TypeIdent, usize>,
    by_namespace_function: HashMap<(String, RustFunctionId), usize>,
    by_imported_function: HashMap<(String, RustFunctionId), usize>,
    by_namespace_associated_function: HashMap<(String, TypeIdent, RustFunctionId), usize>,
    by_namespace_constant: HashMap<(String, Ident), usize>,
    by_imported_constant: HashMap<Ident, usize>,
    by_namespace_associated_constant: HashMap<(String, TypeIdent, Ident), usize>,
}

impl RustInterop {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_uses(uses: &[ToplevelItem<UseDecl>]) -> Result<Self, RustdocError> {
        Self::from_crates_and_uses(imported_crates(uses), uses)
    }

    pub fn from_crates_and_uses(
        crate_names: impl IntoIterator<Item = String>,
        uses: &[ToplevelItem<UseDecl>],
    ) -> Result<Self, RustdocError> {
        let mut interop = RustInterop::default();
        let imported_crates = imported_crates(uses);
        let crate_names = crate_names
            .into_iter()
            .chain(imported_crates.iter().cloned())
            .collect::<HashSet<_>>();

        for crate_name in crate_names {
            let cache = RustdocCache::new(&crate_name);
            cache.update_if_needed();
            if let Some(path) = cache.json_path() {
                let json = fs::read_to_string(&path)
                    .map_err(|error| RustdocError::ReadCache(path.clone(), error))?;
                let json = serde_json::from_str(&json)
                    .map_err(|error| RustdocError::ParseCache(path.clone(), error))?;
                interop.add_crate(&crate_name, &json);
            } else {
                interop.add_curated_crate(&crate_name);
            }
        }
        interop.import_uses(uses);

        Ok(interop)
    }

    pub fn add_function_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: FnDecl,
        borrowed_return: bool,
    ) {
        self.push_function(
            namespace,
            name,
            rust_path.into(),
            decl,
            borrowed_return,
            RustReturnConversion::None,
            Vec::new(),
        );
    }

    pub fn add_type_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: TypeDecl,
    ) {
        let type_decl = RustTypeDecl {
            namespace: namespace.into(),
            name: TypeIdent::new(name),
            rust_path: rust_path.into(),
            field_conversions: Vec::new(),
            constructor_arg_conversions: Vec::new(),
            enum_variant_conversions: Vec::new(),
            decl: ToplevelItem {
                item: decl,
                source: Source::Builtin,
            },
        };
        if let Some(existing) = self.types.iter_mut().find(|ty| ty.name.as_str() == name) {
            *existing = type_decl;
        } else {
            self.types.push(type_decl);
        }
    }

    pub fn add_constant_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        ty: TypeElement,
    ) {
        self.push_constant(namespace, None, name, rust_path.into(), ty);
    }

    pub fn add_associated_constant_decl(
        &mut self,
        namespace: &str,
        receiver: TypeIdent,
        name: &str,
        rust_path: impl Into<Box<str>>,
        ty: TypeElement,
    ) {
        self.push_constant(namespace, Some(receiver), name, rust_path.into(), ty);
    }

    pub fn add_associated_function_decl(
        &mut self,
        namespace: &str,
        receiver: TypeIdent,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: FnDecl,
        borrowed_return: bool,
    ) {
        self.push_function_with_associated_receiver(
            namespace,
            name,
            rust_path.into(),
            decl,
            borrowed_return,
            Some(receiver),
            RustReturnConversion::None,
            Vec::new(),
        );
    }

    pub fn function(
        &self,
        namespace: Option<&str>,
        receiver: Option<&TypeIdent>,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&RustFunctionDecl> {
        let id = RustFunctionId::new(receiver, name.as_str(), labels);
        if let Some(namespace) = namespace {
            return self
                .by_namespace_function
                .get(&(namespace.to_string(), id))
                .and_then(|idx| self.functions.get(*idx));
        }

        self.by_imported_function
            .get(&("".to_string(), id))
            .and_then(|idx| self.functions.get(*idx))
    }

    pub fn associated_function(
        &self,
        namespace: Option<&str>,
        receiver: &TypeIdent,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&RustFunctionDecl> {
        let id = RustFunctionId::new(None, name.as_str(), labels);
        if let Some(namespace) = namespace {
            return self
                .by_namespace_associated_function
                .get(&(namespace.to_string(), receiver.clone(), id))
                .and_then(|idx| self.functions.get(*idx));
        }

        self.by_namespace_associated_function
            .iter()
            .find(|((_, stored_receiver, stored_id), _)| {
                stored_receiver == receiver && stored_id == &id
            })
            .and_then(|(_, idx)| self.functions.get(*idx))
    }

    pub fn imported_types(&self) -> impl Iterator<Item = &RustTypeDecl> {
        self.by_imported_type
            .values()
            .filter_map(|idx| self.types.get(*idx))
    }

    pub fn field_return_conversion(
        &self,
        receiver: &TypeIdent,
        field: &Ident,
    ) -> RustReturnConversion {
        self.types
            .iter()
            .find(|ty| ty.name == *receiver)
            .and_then(|ty| {
                ty.field_conversions
                    .iter()
                    .find(|conversion| conversion.field == *field)
            })
            .map(|conversion| conversion.return_conversion)
            .unwrap_or_default()
    }

    pub fn field_arg_conversion(&self, receiver: &TypeIdent, field: &Ident) -> RustArgConversion {
        self.types
            .iter()
            .find(|ty| ty.name == *receiver)
            .and_then(|ty| {
                ty.field_conversions
                    .iter()
                    .find(|conversion| conversion.field == *field)
            })
            .map(|conversion| conversion.arg_conversion)
            .unwrap_or_default()
    }

    pub fn constructor_arg_conversions(&self, receiver: &TypeIdent) -> Vec<RustArgConversion> {
        self.types
            .iter()
            .find(|ty| ty.name == *receiver)
            .map(|ty| ty.constructor_arg_conversions.clone())
            .unwrap_or_default()
    }

    pub fn enum_variant_arg_conversion(
        &self,
        receiver: &TypeIdent,
        variant: &TypeIdent,
        index: usize,
        field: Option<&Ident>,
    ) -> RustArgConversion {
        self.enum_variant_conversion(receiver, variant, index, field)
            .map(|conversion| conversion.arg_conversion)
            .unwrap_or_default()
    }

    pub fn enum_variant_return_conversion(
        &self,
        receiver: &TypeIdent,
        variant: &TypeIdent,
        index: usize,
        field: Option<&Ident>,
    ) -> RustReturnConversion {
        self.enum_variant_conversion(receiver, variant, index, field)
            .map(|conversion| conversion.return_conversion)
            .unwrap_or_default()
    }

    fn enum_variant_conversion(
        &self,
        receiver: &TypeIdent,
        variant: &TypeIdent,
        index: usize,
        field: Option<&Ident>,
    ) -> Option<&RustEnumVariantArgConversion> {
        self.types
            .iter()
            .find(|ty| ty.name == *receiver)
            .and_then(|ty| {
                ty.enum_variant_conversions
                    .iter()
                    .find(|conversion| conversion.variant == *variant)
            })
            .and_then(|conversion| {
                if let Some(field) = field {
                    return conversion
                        .args
                        .iter()
                        .find(|arg| arg.field.as_ref() == Some(field));
                }
                conversion.args.get(index)
            })
    }

    pub fn constant(&self, namespace: Option<&str>, name: &Ident) -> Option<&RustConstantDecl> {
        if let Some(namespace) = namespace {
            return self
                .by_namespace_constant
                .get(&(namespace.to_string(), name.clone()))
                .and_then(|idx| self.constants.get(*idx));
        }

        self.by_imported_constant
            .get(name)
            .and_then(|idx| self.constants.get(*idx))
    }

    pub fn associated_constant(
        &self,
        namespace: Option<&str>,
        receiver: &TypeIdent,
        name: &Ident,
    ) -> Option<&RustConstantDecl> {
        if let Some(namespace) = namespace {
            return self
                .by_namespace_associated_constant
                .get(&(namespace.to_string(), receiver.clone(), name.clone()))
                .and_then(|idx| self.constants.get(*idx));
        }

        self.by_namespace_associated_constant
            .iter()
            .find(|((_, stored_receiver, stored_name), _)| {
                stored_receiver == receiver && stored_name == name
            })
            .and_then(|(_, idx)| self.constants.get(*idx))
    }

    fn push_type(&mut self, crate_name: &str, name: &str) {
        if self.types.iter().any(|ty| ty.name.as_str() == name) {
            return;
        }

        let ident = TypeIdent::new(name);
        self.types.push(RustTypeDecl {
            namespace: crate_name.into(),
            name: ident.clone(),
            rust_path: format!("::{crate_name}::{name}").into(),
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

    fn push_type_from_item(
        &mut self,
        crate_name: &str,
        item: &Value,
        index: &serde_json::Map<String, Value>,
    ) {
        let Some(name) = public_type_name(item) else {
            return;
        };

        let imported = self
            .type_decl_from_item(crate_name, name, item, index)
            .unwrap_or_else(|| ImportedTypeDecl::empty(name));
        let rust_path = rust_path(crate_name, name, item);
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

        if let Some(existing) = self.types.iter_mut().find(|ty| ty.name.as_str() == name) {
            if matches!(existing.decl.item, TypeDecl::Empty(_)) {
                *existing = type_decl;
            }
            return;
        }

        self.types.push(type_decl);
    }

    fn push_reexported_type_from_item(
        &mut self,
        crate_name: &str,
        exported_name: &str,
        rust_path: Box<str>,
        item: &Value,
        index: &serde_json::Map<String, Value>,
    ) {
        if self
            .types
            .iter()
            .any(|ty| ty.name.as_str() == exported_name)
        {
            if let Some(existing) = self
                .types
                .iter_mut()
                .find(|ty| ty.name.as_str() == exported_name)
            {
                existing.rust_path = rust_path;
            }
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
    fn push_function(
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

    fn push_constant(
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

    fn push_function_with_associated_receiver(
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

fn imported_crates(uses: &[ToplevelItem<UseDecl>]) -> HashSet<String> {
    uses.iter()
        .filter_map(|use_decl| use_decl.path.segments.first())
        .map(|segment| segment.as_str().to_string())
        .collect()
}

#[cfg(test)]
mod tests;
