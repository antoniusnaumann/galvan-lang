use galvan_ast::{Ident, TypeIdent};

use crate::model::{
    RustArgConversion, RustConstantDecl, RustEnumVariantArgConversion, RustFunctionDecl,
    RustReturnConversion, RustTypeDecl,
};

use super::function_id::RustFunctionId;
use super::RustInterop;

impl RustInterop {
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
