use typeunion::type_union;

use crate::{Ident, TypeIdent};

type Array = Box<ArrayTypeItem>;
type Dictionary = Box<DictionaryTypeItem>;
type OrderedDictionary = Box<OrderedDictionaryTypeItem>;
type Set = Box<SetTypeItem>;
type Tuple = Box<TupleTypeItem>;
type Optional = Box<OptionalTypeItem>;
type Result = Box<ResultTypeItem>;
type Plain = BasicTypeItem;
type Generic = GenericTypeItem;

#[type_union]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub type TypeElement =
    Array + Dictionary + OrderedDictionary + Set + Tuple + Optional + Result + Plain + Generic;

impl From<TypeIdent> for TypeElement {
    fn from(value: TypeIdent) -> Self {
        Self::Plain(BasicTypeItem { ident: value })
    }
}

impl TypeElement {
    pub fn plain(ident: TypeIdent) -> Self {
        Self::Plain(BasicTypeItem { ident })
    }

    pub fn array(elements: TypeElement) -> Self {
        Self::Array(Box::new(ArrayTypeItem { elements }))
    }

    pub fn dict(key: TypeElement, value: TypeElement) -> Self {
        Self::Dictionary(Box::new(DictionaryTypeItem { key, value }))
    }

    pub fn ordered_dict(key: TypeElement, value: TypeElement) -> Self {
        Self::OrderedDictionary(Box::new(OrderedDictionaryTypeItem { key, value }))
    }

    pub fn set(elements: TypeElement) -> Self {
        Self::Set(Box::new(SetTypeItem { elements }))
    }

    pub fn tuple(elements: Vec<TypeElement>) -> Self {
        Self::Tuple(Box::new(TupleTypeItem { elements }))
    }

    pub fn optional(some: TypeElement) -> Self {
        Self::Optional(Box::new(OptionalTypeItem { some }))
    }

    pub fn result(success: TypeElement, error: Option<TypeElement>) -> Self {
        Self::Result(Box::new(ResultTypeItem { success, error }))
    }
}

// TODO: Add a marker trait to constrain this to only type decls
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArrayTypeItem {
    pub elements: TypeElement,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DictionaryTypeItem {
    pub key: TypeElement,
    pub value: TypeElement,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderedDictionaryTypeItem {
    pub key: TypeElement,
    pub value: TypeElement,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SetTypeItem {
    pub elements: TypeElement,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TupleTypeItem {
    pub elements: Vec<TypeElement>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OptionalTypeItem {
    pub some: TypeElement,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResultTypeItem {
    pub success: TypeElement,
    pub error: Option<TypeElement>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BasicTypeItem {
    pub ident: TypeIdent,
    // TODO: Handle generics
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericTypeItem {
    pub ident: Ident,
    // TODO: Handle generics
}

