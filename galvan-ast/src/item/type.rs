use derive_more::From;
use galvan_pest::Rule;

use super::{Ident, TypeIdent, TypeElement, Visibility};

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::type_decl))]
pub enum TypeDecl {
    Tuple(TupleTypeDecl),
    Struct(StructTypeDecl),
    Alias(AliasTypeDecl),
    Empty(EmptyTypeDecl),
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
pub struct EmptyTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
}


#[derive(Debug, PartialEq, Eq)]
pub struct ReceiverType {}
#[derive(Debug, PartialEq, Eq,)]
pub struct ReturnType {}
#[derive(Debug, PartialEq, Eq,)]
pub struct ParamType {}