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
    pub r#type: TypeItem<BasicTypeItem>,
}

#[derive(Debug)]
pub struct StructTypeDecl {
    pub members: Vec<StructTypeMember>,
}
#[derive(Debug)]
pub struct StructTypeMember {
    pub visibility: Visibility,
    pub ident: Ident,
    pub r#type: TypeItem<BasicTypeItem>,
}

#[derive(Debug)]
pub struct AliasTypeDecl {
    pub r#type: TypeItem<BasicTypeItem>,
}

#[derive(Debug)]
pub enum TypeItem<T> {
    Array(Box<ArrayTypeItem<T>>),
    Dictionary(Box<DictionaryTypeItem<T>>),
    Tuple(Box<TupleTypeItem<T>>),
    Plain(T),
}

impl TypeItem<BasicTypeItem> {
    pub fn plain(ident: LexerString) -> Self {
        Self::Plain(BasicTypeItem {
            ident: Ident::new(ident),
        })
    }
}

// TODO: Add a marker trait to constrain this to only type decls
#[derive(Debug)]
pub struct ArrayTypeItem<T> {
    pub elements: TypeItem<T>,
}

#[derive(Debug)]
pub struct DictionaryTypeItem<T> {
    pub key: TypeItem<T>,
    pub value: TypeItem<T>,
}

#[derive(Debug)]
pub struct TupleTypeItem<T> {
    pub elements: Vec<TypeItem<T>>,
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
