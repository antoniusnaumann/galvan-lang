use galvan_ast::{FnDecl, Ident, ToplevelItem, TypeDecl, TypeElement, TypeIdent};

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RustArgConversion {
    #[default]
    None,
    SharedBorrow,
    BoxNew,
    RcNew,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RustReturnConversion {
    #[default]
    None,
    BoxDeref,
    RcCloneDeref,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustFieldConversion {
    pub field: Ident,
    pub arg_conversion: RustArgConversion,
    pub return_conversion: RustReturnConversion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustEnumVariantConversion {
    pub variant: TypeIdent,
    pub args: Vec<RustEnumVariantArgConversion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustEnumVariantArgConversion {
    pub field: Option<Ident>,
    pub arg_conversion: RustArgConversion,
}

#[derive(Debug)]
pub struct RustConstantDecl {
    pub namespace: Box<str>,
    pub associated_receiver: Option<TypeIdent>,
    pub name: Ident,
    pub rust_path: Box<str>,
    pub ty: TypeElement,
}
