use crate::*;

pub enum TypeDecl {
    TupleType(TupleTypeDecl),
    StructType(StructTypeDecl),
    AliasType(AliasTypeDecl),
}

pub struct TupleTypeDecl {
    pub members: Vec<TypeItem<BasicTypeItem>>,
}
pub struct StructTypeDecl {}
pub struct AliasTypeDecl {}

pub enum TypeItem<T> {
    Array(Box<ArrayTypeItem<T>>),
    Dictionary(Box<DictionaryTypeItem<T>>),
    Tuple(Box<TupleTypeItem<T>>),
    Plain(T),
}

// TODO: Add a marker trait to constrain this to only type decls
pub struct ArrayTypeItem<T> {
    pub elements: TypeItem<T>,
}

pub struct DictionaryTypeItem<T> {
    pub key: TypeItem<T>,
    pub value: TypeItem<T>,
}

pub struct TupleTypeItem<T> {
    pub elements: Vec<TypeItem<T>>,
}

pub struct BasicTypeItem {
    pub ident: Ident,
    // TODO: Handle generics
}

pub struct ReceiverType {}
pub struct ReturnType {}
pub struct ParamType {}
