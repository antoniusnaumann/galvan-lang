use derive_more::From;
use galvan_ast_macro::AstNode;

use crate::{AstNode, PrintAst, Span};

use super::{DeclModifier, Ident, TypeElement, TypeIdent, Visibility};

#[derive(Debug, PartialEq, Eq, From)]
pub enum TypeDecl {
    Tuple(TupleTypeDecl),
    Struct(StructTypeDecl),
    Alias(AliasTypeDecl),
    Enum(EnumTypeDecl),
    Empty(EmptyTypeDecl),
}

impl TypeDecl {
    pub fn ident(&self) -> &TypeIdent {
        match self {
            TypeDecl::Tuple(t) => &t.ident,
            TypeDecl::Struct(s) => &s.ident,
            TypeDecl::Alias(a) => &a.ident,
            TypeDecl::Empty(e) => &e.ident,
            TypeDecl::Enum(e) => &e.ident,
        }
    }
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct TupleTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub members: Vec<TupleTypeMember>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct TupleTypeMember {
    // pub visibility: Visibility,
    pub r#type: TypeElement,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct StructTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub members: Vec<StructTypeMember>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct StructTypeMember {
    // pub visibility: Visibility,
    pub decl_modifier: Option<DeclModifier>,
    pub ident: Ident,
    pub r#type: TypeElement,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct AliasTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub r#type: TypeElement,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct EnumTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub members: Vec<EnumTypeMember>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct EnumTypeMember {
    pub ident: TypeIdent,
    pub fields: Vec<EnumVariantField>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct EnumVariantField {
    pub name: Option<Ident>,  // None for anonymous fields
    pub r#type: TypeElement,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
/// An empty struct without any fields e.g.: `type Empty`
pub struct EmptyTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub span: Span,
}
