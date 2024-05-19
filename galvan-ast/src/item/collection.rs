use crate::{AstNode, Expression, PrintAst, Span};
use galvan_ast_macro::{AstNode, PrintAst};
use typeunion::type_union;

#[type_union]
#[derive(Debug, PartialEq, Eq, AstNode)]
pub type CollectionLiteral = ArrayLiteral + DictLiteral + SetLiteral + OrderedDictLiteral;

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct ArrayLiteral {
    pub elements: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct DictLiteral {
    pub elements: Vec<DictLiteralElement>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct DictLiteralElement {
    pub key: Expression,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct SetLiteral {
    pub elements: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct OrderedDictLiteral {
    pub elements: Vec<DictLiteralElement>,
    pub span: Span,
}
