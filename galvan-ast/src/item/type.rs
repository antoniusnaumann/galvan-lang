use derive_more::From;

use super::{DeclModifier, Ident, TypeElement, TypeIdent, Visibility};

#[derive(Debug, PartialEq, Eq, From)]
pub enum TypeDecl {
    Tuple(TupleTypeDecl),
    Struct(StructTypeDecl),
    Alias(AliasTypeDecl),
    Empty(EmptyTypeDecl),
}

impl TypeDecl {
    pub fn ident(&self) -> &TypeIdent {
        match self {
            TypeDecl::Tuple(t) => &t.ident,
            TypeDecl::Struct(s) => &s.ident,
            TypeDecl::Alias(a) => &a.ident,
            TypeDecl::Empty(e) => &e.ident,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TupleTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub members: Vec<TupleTypeMember>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TupleTypeMember {
    // pub visibility: Visibility,
    pub r#type: TypeElement,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StructTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub members: Vec<StructTypeMember>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StructTypeMember {
    // pub visibility: Visibility,
    pub decl_modifier: Option<DeclModifier>,
    pub ident: Ident,
    pub r#type: TypeElement,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AliasTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub r#type: TypeElement,
}

#[derive(Debug, PartialEq, Eq)]
/// An empty struct without any fields e.g.: `type Empty`
pub struct EmptyTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
}
