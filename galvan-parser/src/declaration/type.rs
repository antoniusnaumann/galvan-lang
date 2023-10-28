use derive_more::From;
use galvan_lexer::LexerString;

use crate::*;

#[derive(Debug)]
pub enum TypeDecl {
    TupleType(TupleTypeDecl),
    StructType(StructTypeDecl),
    AliasType(AliasTypeDecl),
}

#[derive(Debug)]
pub struct TupleTypeDecl {
    pub members: Vec<TupleTypeMember>,
}

#[derive(Debug)]
pub struct TupleTypeMember {
    pub visibility: Visibility,
    pub r#type: TypeItem,
}

#[derive(Debug)]
pub struct StructTypeDecl {
    pub members: Vec<StructTypeMember>,
}
#[derive(Debug)]
pub struct StructTypeMember {
    pub visibility: Visibility,
    pub ident: Ident,
    pub r#type: TypeItem,
}

#[derive(Debug)]
pub struct AliasTypeDecl {
    pub r#type: TypeItem,
}

#[derive(Debug, From)]
pub enum TypeItem {
    Array(Box<ArrayTypeItem>),
    Dictionary(Box<DictionaryTypeItem>),
    Set(Box<SetTypeItem>),
    Tuple(Box<TupleTypeItem>),
    Plain(BasicTypeItem),
}

impl TypeItem {
    pub fn plain(ident: LexerString) -> Self {
        Self::Plain(BasicTypeItem {
            ident: Ident::new(ident),
        })
    }
}

// TODO: Add a marker trait to constrain this to only type decls
#[derive(Debug)]
pub struct ArrayTypeItem {
    pub elements: TypeItem,
}

#[derive(Debug)]
pub struct DictionaryTypeItem {
    pub key: TypeItem,
    pub value: TypeItem,
}

#[derive(Debug)]
pub struct SetTypeItem {
    pub elements: TypeItem,
}

#[derive(Debug)]
pub struct TupleTypeItem {
    pub elements: Vec<TypeItem>,
}

#[derive(Debug)]
pub struct BasicTypeItem {
    pub ident: Ident,
    // TODO: Handle generics
}

#[derive(Debug)]
pub struct ReceiverType {}
#[derive(Debug)]
pub struct ReturnType {}
#[derive(Debug)]
pub struct ParamType {}
