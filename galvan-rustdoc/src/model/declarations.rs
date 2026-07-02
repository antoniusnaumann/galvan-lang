use galvan_ast::{FnDecl, Ident, ToplevelItem, TypeDecl, TypeElement, TypeIdent};

use super::{
    RustArgConversion, RustEnumVariantConversion, RustFieldConversion, RustReturnConversion,
};

#[derive(Debug)]
pub struct RustTypeDecl {
    pub namespace: Box<str>,
    pub name: TypeIdent,
    pub rust_path: Box<str>,
    pub field_conversions: Vec<RustFieldConversion>,
    pub constructor_arg_conversions: Vec<RustArgConversion>,
    pub enum_variant_conversions: Vec<RustEnumVariantConversion>,
    pub decl: ToplevelItem<TypeDecl>,
}

#[derive(Debug)]
pub struct RustFunctionDecl {
    pub namespace: Box<str>,
    pub rust_path: Box<str>,
    pub borrowed_return: bool,
    pub return_conversion: RustReturnConversion,
    pub arg_conversions: Vec<RustArgConversion>,
    pub decl: ToplevelItem<FnDecl>,
}

#[derive(Debug)]
pub struct RustConstantDecl {
    pub namespace: Box<str>,
    pub associated_receiver: Option<TypeIdent>,
    pub name: Ident,
    pub rust_path: Box<str>,
    pub ty: TypeElement,
}
