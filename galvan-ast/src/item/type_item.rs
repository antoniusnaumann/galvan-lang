use galvan_ast_macro::AstNode;
use typeunion::type_union;

use crate::{AstNode, Ident, PrintAst, Span, TypeIdent};

type Array = Box<ArrayTypeItem>;
type Dictionary = Box<DictionaryTypeItem>;
type OrderedDictionary = Box<OrderedDictionaryTypeItem>;
type Set = Box<SetTypeItem>;
type Tuple = Box<TupleTypeItem>;
type Optional = Box<OptionalTypeItem>;
type Result = Box<ResultTypeItem>;
type Plain = BasicTypeItem;
type Generic = GenericTypeItem;
type Never = NeverTypeItem;

#[type_union]
#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub type TypeElement = Array
    + Dictionary
    + OrderedDictionary
    + Set
    + Tuple
    + Optional
    + Result
    + Plain
    + Generic
    + Never;

// TODO: Add a marker trait to constrain this to only type decls
#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct ArrayTypeItem {
    pub elements: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct DictionaryTypeItem {
    pub key: TypeElement,
    pub value: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct OrderedDictionaryTypeItem {
    pub key: TypeElement,
    pub value: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct SetTypeItem {
    pub elements: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct TupleTypeItem {
    pub elements: Vec<TypeElement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct OptionalTypeItem {
    pub inner: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct ResultTypeItem {
    pub success: TypeElement,
    pub error: Option<TypeElement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct BasicTypeItem {
    pub ident: TypeIdent,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct GenericTypeItem {
    pub ident: Ident,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct NeverTypeItem {
    pub span: Span,
}
