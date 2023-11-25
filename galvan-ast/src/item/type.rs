use derive_more::From;
use galvan_pest::Rule;

use super::{Ident, TypeIdent, Visibility};

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
#[pest_ast(rule(Rule::element_type))]
pub struct TupleTypeMember {
    // pub visibility: Visibility,
    pub r#type: TypeItem,
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
    pub r#type: TypeItem,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::alias_type_decl))]
pub struct AliasTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
    pub r#type: TypeItem,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::empty_type_decl))]
pub struct EmptyTypeDecl {
    pub visibility: Visibility,
    pub ident: TypeIdent,
}

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::type_item))]
pub enum TypeItem {
    // Collection Types
    Array(Box<ArrayTypeItem>),
    Dictionary(Box<DictionaryTypeItem>),
    OrderedDictionary(Box<OrderedDictionaryTypeItem>),
    Set(Box<SetTypeItem>),
    Tuple(Box<TupleTypeItem>),

    // Error handling monads
    Optional(Box<OptionalTypeItem>),
    Result(Box<ResultTypeItem>),

    // Primitive type
    Plain(BasicTypeItem),
}

impl From<TypeIdent> for TypeItem {
    fn from(value: TypeIdent) -> Self {
        Self::Plain(BasicTypeItem { ident: value })
    }
}

impl TypeItem {
    pub fn plain(ident: TypeIdent) -> Self {
        Self::Plain(BasicTypeItem { ident })
    }

    pub fn array(elements: TypeItem) -> Self {
        Self::Array(Box::new(ArrayTypeItem { elements }))
    }

    pub fn dict(key: TypeItem, value: TypeItem) -> Self {
        Self::Dictionary(Box::new(DictionaryTypeItem { key, value }))
    }

    pub fn ordered_dict(key: TypeItem, value: TypeItem) -> Self {
        Self::OrderedDictionary(Box::new(OrderedDictionaryTypeItem { key, value }))
    }

    pub fn set(elements: TypeItem) -> Self {
        Self::Set(Box::new(SetTypeItem { elements }))
    }

    pub fn tuple(elements: Vec<TypeItem>) -> Self {
        Self::Tuple(Box::new(TupleTypeItem { elements }))
    }

    pub fn optional(some: TypeItem) -> Self {
        Self::Optional(Box::new(OptionalTypeItem { some }))
    }

    pub fn result(success: TypeItem) -> Self {
        Self::Result(Box::new(ResultTypeItem {
            success,
            error: None,
        }))
    }

    pub fn result_with_typed_error(success: TypeItem, error: TypeItem) -> Self {
        Self::Result(Box::new(ResultTypeItem {
            success,
            error: Some(error),
        }))
    }
}

// TODO: Add a marker trait to constrain this to only type decls
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::array_type))]
pub struct ArrayTypeItem {
    pub elements: TypeItem,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::dict_type))]
pub struct DictionaryTypeItem {
    pub key: TypeItem,
    pub value: TypeItem,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::ordered_dict_type))]
pub struct OrderedDictionaryTypeItem {
    pub key: TypeItem,
    pub value: TypeItem,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::set_type))]
pub struct SetTypeItem {
    pub elements: TypeItem,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::tuple_type))]
pub struct TupleTypeItem {
    pub elements: Vec<TypeItem>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::optional_type))]
pub struct OptionalTypeItem {
    pub some: TypeItem,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::result_type))]
pub struct ResultTypeItem {
    pub success: TypeItem,
    pub error: Option<TypeItem>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::basic_type))]
pub struct BasicTypeItem {
    pub ident: TypeIdent,
    // TODO: Handle generics
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReceiverType {}
#[derive(Debug, PartialEq, Eq,)]
pub struct ReturnType {}
#[derive(Debug, PartialEq, Eq,)]
pub struct ParamType {}
