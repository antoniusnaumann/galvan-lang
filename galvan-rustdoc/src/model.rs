use galvan_ast::{FnDecl, Ident, ToplevelItem, TypeDecl, TypeElement, TypeIdent};

#[derive(Debug)]
pub struct RustTypeDecl {
    pub namespace: Box<str>,
    pub name: TypeIdent,
    pub rust_path: Box<str>,
    pub decl: ToplevelItem<TypeDecl>,
}

#[derive(Debug)]
pub struct RustFunctionDecl {
    pub namespace: Box<str>,
    pub rust_path: Box<str>,
    pub borrowed_return: bool,
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

#[derive(Debug)]
pub struct RustConstantDecl {
    pub namespace: Box<str>,
    pub associated_receiver: Option<TypeIdent>,
    pub name: Ident,
    pub rust_path: Box<str>,
    pub ty: TypeElement,
}
