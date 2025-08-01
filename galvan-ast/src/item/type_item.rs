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
type Void = VoidTypeItem;
type Infer = InferTypeItem;
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
    + Infer
    + Void
    + Never;

impl TypeElement {
    pub fn bool() -> Self {
        BasicTypeItem {
            ident: TypeIdent::new("Bool"),
            span: Span::default(),
        }
        .into()
    }

    pub fn infer() -> Self {
        InferTypeItem::default().into()
    }

    pub fn void() -> Self {
        VoidTypeItem::default().into()
    }
}

impl Default for TypeElement {
    fn default() -> Self {
        Self::infer()
    }
}

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

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, AstNode)]
pub struct NeverTypeItem {
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, AstNode)]
pub struct VoidTypeItem {
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, AstNode)]
pub struct InferTypeItem {
    pub span: Span,
}
