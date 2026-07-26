use galvan_ast::{
    AstNode, FnDecl, Ident, SegmentedAsts, Span, ToplevelItem, TypeDecl, TypeElement, TypeIdent,
};
use galvan_files::Source;
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
}

/// What kind of top-level declaration a [`DuplicateDeclaration`] refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarationKind {
    Type,
    Function,
}

impl std::fmt::Display for DeclarationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeclarationKind::Type => write!(f, "type"),
            DeclarationKind::Function => write!(f, "function"),
        }
    }
}

/// A top-level declaration that conflicts with an earlier one of the same
/// name. The earlier declaration stays in the [`LookupContext`]; the duplicate
/// is reported so that callers can surface it as a diagnostic and continue.
#[derive(Clone, Debug)]
pub struct DuplicateDeclaration {
    pub kind: DeclarationKind,
    pub name: String,
    /// Identifier span of the duplicate declaration.
    pub span: Span,
    /// Source file of the duplicate declaration.
    pub source: Source,
}

impl<'a> LookupContext<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers all top-level declarations from `asts`.
    ///
    /// Conflicting redeclarations do not abort: the first declaration wins and
    /// each conflict is returned as a [`DuplicateDeclaration`].
    pub fn add_from(&mut self, asts: &'a SegmentedAsts) -> Vec<DuplicateDeclaration> {
        let mut duplicates = Vec::new();

        for func in &asts.functions {
            let receiver = func.item.signature.parameters.params.first().and_then(|p| {
                if p.identifier.is_self() {
                    // TODO: We should allow implementing something on Vec, etc. as well
                    match p.param_type {
                        TypeElement::Plain(ref ty) => Some(&ty.ident),
                        _ => None,
                    }
                } else {
                    None
                }
            });
            let labels = func.item.signature.overload_labels();
            let labels = labels
                .iter()
                .map(|label| label.as_str())
                .collect::<Vec<_>>();
            let func_id = FunctionId::new(receiver, &func.signature.identifier, &labels);
            if self.functions.contains_key(&func_id) {
                duplicates.push(DuplicateDeclaration {
                    kind: DeclarationKind::Function,
                    name: func_id.to_string(),
                    span: func.signature.identifier.span(),
                    source: func.source.clone(),
                });
            } else {
                self.functions.insert(func_id, func);
            }
        }

        for type_decl in &asts.types {
            let ident = type_decl.ident();
            if self.types.contains_key(ident) {
                duplicates.push(DuplicateDeclaration {
                    kind: DeclarationKind::Type,
                    name: ident.to_string(),
                    span: ident.span(),
                    source: type_decl.source.clone(),
                });
            } else {
                self.types.insert(ident.clone(), type_decl);
            }
        }

        duplicates
    }

    /// Like [`add_from`](Self::add_from), but discards conflict information.
    /// Intended for contexts (builtins, already-checked modules) where
    /// duplicates have been reported elsewhere or cannot occur.
    pub fn with(mut self, asts: &'a SegmentedAsts) -> Self {
        self.add_from(asts);
        self
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
