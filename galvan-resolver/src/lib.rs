use std::collections::HashMap;

use thiserror::Error;

use galvan_ast::{FnDecl, Ident, MainDecl, SegmentedAsts, ToplevelItem, TypeDecl, TypeIdent};

#[derive(Debug, Default)]
pub struct LookupContext<'a> {
    /// Types are resolved by their name
    pub types: HashMap<TypeIdent, &'a ToplevelItem<TypeDecl>>,
    /// Functions are resolved by their name and - if present - named arguments and their receiver type
    ///
    /// `fn foo(a: i32, b: i32) -> i32` is identified as `foo`
    /// `fn foo(bar a: i32, b: i32) -> i32` is identified as `foo:bar`
    /// `fn foo(self: i32, b: i32) -> i32` is identified as `i32::foo`
    pub functions: HashMap<FunctionId, &'a ToplevelItem<FnDecl>>,
    // TODO: Nested contexts for resolving names from imported modules
    // pub imports: HashMap<String, LookupContext<'a>>,
    pub main: Option<&'a ToplevelItem<MainDecl>>,
}

// TODO: derive thiserror and add proper error handling #[derive(Error)]
// TODO: Include spans in errors
#[derive(Debug, Error)]
pub enum LookupError {
    #[error("Type not found")]
    TypeNotFound,
    #[error("Function not found")]
    FunctionNotFound,
    #[error("Duplicate type")]
    DuplicateType(TypeIdent),
    #[error("Duplicate function")]
    DuplicateFunction,
}

impl<'a> LookupContext<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_from(&mut self, asts: &'a SegmentedAsts) -> Result<(), LookupError> {
        for func in &asts.functions {
            let func_id = FunctionId::new(None, &func.signature.identifier, &[]);
            if self.functions.insert(func_id, func).is_some() {
                return Err(LookupError::DuplicateFunction);
            }
        }

        for type_decl in &asts.types {
            if self
                .types
                .insert(type_decl.ident().clone(), type_decl)
                .is_some()
            {
                return Err(LookupError::DuplicateType(type_decl.ident().clone()));
            }
        }

        Ok(())
    }

    pub fn with(mut self, asts: &'a SegmentedAsts) -> Result<Self, LookupError> {
        self.add_from(asts)?;
        Ok(self)
    }
}

impl LookupContext<'_> {
    pub fn resolve_type(&self, name: &TypeIdent) -> Option<&ToplevelItem<TypeDecl>> {
        self.types.get(&name).copied()
    }

    pub fn resolve_function(
        &self,
        receiver: Option<&TypeIdent>,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&ToplevelItem<FnDecl>> {
        let func_id = FunctionId::new(receiver, name, labels);
        self.functions.get(&func_id).copied()
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct FunctionId(Box<str>);

impl FunctionId {
    fn new(receiver: Option<&TypeIdent>, fn_ident: &Ident, labels: &[&str]) -> Self {
        let mut id = String::new();
        if let Some(receiver) = receiver {
            id.push_str(receiver.as_str());
            id.push_str("::");
        }
        id.push_str(fn_ident.as_str());
        if !labels.is_empty() {
            id.push(':');
            id.push_str(&labels.join(":"));
        }

        Self(id.into())
    }
}
