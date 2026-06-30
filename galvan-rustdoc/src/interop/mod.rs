use std::collections::{HashMap, HashSet};
use std::fs;

use galvan_ast::{FnDecl, Ident, ToplevelItem, TypeDecl, TypeElement, TypeIdent, UseDecl};
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
mod registry;
mod rustdoc_json;
mod uses;

use self::function_id::RustFunctionId;
use self::uses::imported_crates;

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
        let rust_path = rust_path.into();
        let type_decl = RustTypeDecl {
            namespace: namespace.into(),
            name: TypeIdent::new(name),
            rust_path,
            field_conversions: Vec::new(),
            constructor_arg_conversions: Vec::new(),
            enum_variant_conversions: Vec::new(),
            decl: ToplevelItem {
                item: decl,
                source: Source::Builtin,
            },
        };
        if let Some(existing) = self
            .types
            .iter_mut()
            .find(|ty| ty.rust_path.as_ref() == type_decl.rust_path.as_ref())
        {
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

    pub fn type_by_qualified_path(&self, path: &[&str]) -> Option<&RustTypeDecl> {
        let namespace = path.first()?;
        let (name, _) = path.split_last()?;
        let rust_path = format!("::{}", path.join("::"));
        self.types.iter().find(|ty| {
            ty.namespace.as_ref() == *namespace
                && ty.name.as_str() == *name
                && ty.rust_path.as_ref() == rust_path
        })
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
}

#[cfg(test)]
mod tests;
