use derive_more::From;
use galvan_pest::Rule;

use super::{DeclModifier, Ident, TypeElement, TypeIdent, Visibility};

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::type_decl))]
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

    pub fn extern_name(&self) -> &str {
        // TODO: Decide how to make Galvan aware of Rust types
        // TODO:    - Use a @extern("some_name") annotation (how to differentiate between "do not create this type but use the Rust type" and "create this type and customize the Rust name"?)
        // TODO:    - Use a specialized syntax for extern types
        self.ident().as_str()
    }
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::tuple_type_decl))]
pub struct TupleTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub members: Vec<TupleTypeMember>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::tuple_field))]
pub struct TupleTypeMember {
    // pub visibility: Visibility,
    pub r#type: TypeElement,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::struct_type_decl))]
pub struct StructTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub members: Vec<StructTypeMember>,
}
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::struct_field))]
pub struct StructTypeMember {
    // pub visibility: Visibility,
    pub decl_modifier: DeclModifier,
    pub ident: Ident,
    pub r#type: TypeElement,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::alias_type_decl))]
pub struct AliasTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub r#type: TypeElement,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::empty_type_decl))]
/// An empty struct without any fields e.g.: `type Empty`
pub struct EmptyTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
}
