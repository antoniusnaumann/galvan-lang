use galvan_ast::{
    FnDecl, Ident, MainDecl, SegmentedAsts, ToplevelItem, TypeDecl, TypeElement, TypeIdent,
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Clone, Debug, Default)]
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

pub trait Lookup {
    fn resolve_type(&self, name: &TypeIdent) -> Option<&ToplevelItem<TypeDecl>>;

    fn resolve_function(
        &self,
        receiver: Option<&TypeIdent>,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&ToplevelItem<FnDecl>>;
}

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
            let receiver = func.item.signature.parameters.params.first().and_then(|p| {
                if p.identifier.as_str() == "self" {
                    // TODO: We should allow implementing something on Vec, etc. as well
                    match p.param_type {
                        TypeElement::Plain(ref ty) => Some(&ty.ident),
                        _ => None,
                    }
                } else {
                    None
                }
            });
            let func_id = FunctionId::new(receiver, &func.signature.identifier, &[]);
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

impl Lookup for LookupContext<'_> {
    fn resolve_type(&self, name: &TypeIdent) -> Option<&ToplevelItem<TypeDecl>> {
        self.types.get(&name).copied()
    }

    fn resolve_function(
        &self,
        receiver: Option<&TypeIdent>,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&ToplevelItem<FnDecl>> {
        let func_id = FunctionId::new(receiver, name, labels);
        self.functions.get(&func_id).copied()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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

impl std::fmt::Display for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
