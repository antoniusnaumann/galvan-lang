use std::path::PathBuf;

use galvan_ast::{FnDecl, Ident, ToplevelItem, TypeDecl, TypeElement, TypeIdent};

use super::{
    RustArgConversion, RustEnumVariantConversion, RustFieldConversion, RustReturnConversion,
};

/// Location of a lifted item's declaration in its Rust source, taken from
/// rustdoc's span. `line` is 1-based, `column` 0-based (rustdoc conventions).
/// The path is stored as rustdoc emitted it: absolute for registry crates,
/// possibly relative to the consumer project for path dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustSourceSpan {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}

impl RustSourceSpan {
    pub fn from_rustdoc(span: &rustdoc_types::Span) -> Self {
        Self {
            path: span.filename.clone(),
            line: span.begin.0,
            column: span.begin.1,
        }
    }
}

#[derive(Debug)]
pub struct RustTypeDecl {
    pub namespace: Box<str>,
    pub name: TypeIdent,
    pub rust_path: Box<str>,
    pub field_conversions: Vec<RustFieldConversion>,
    pub constructor_arg_conversions: Vec<RustArgConversion>,
    pub enum_variant_conversions: Vec<RustEnumVariantConversion>,
    pub decl: ToplevelItem<TypeDecl>,
    pub source_span: Option<RustSourceSpan>,
}

#[derive(Debug)]
pub struct RustFunctionDecl {
    pub namespace: Box<str>,
    /// The type this function is associated with (`Type.function()` call
    /// form), when it is not a free function.
    pub associated_receiver: Option<TypeIdent>,
    pub rust_path: Box<str>,
    pub extension_trait: Option<Box<str>>,
    pub borrowed_return: bool,
    pub return_conversion: RustReturnConversion,
    pub arg_conversions: Vec<RustArgConversion>,
    pub decl: ToplevelItem<FnDecl>,
    pub source_span: Option<RustSourceSpan>,
}

#[derive(Debug)]
pub struct RustConstantDecl {
    pub namespace: Box<str>,
    pub associated_receiver: Option<TypeIdent>,
    pub name: Ident,
    pub rust_path: Box<str>,
    pub ty: TypeElement,
    pub source_span: Option<RustSourceSpan>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RustdocCrateLiftSummary {
    pub crate_name: Box<str>,
    pub types: usize,
    pub functions: usize,
    pub constants: usize,
}

impl RustdocCrateLiftSummary {
    pub fn total_items(&self) -> usize {
        self.types + self.functions + self.constants
    }
}
