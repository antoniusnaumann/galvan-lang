use crate::{AstNode, Expression, PrintAst, Span};
use derive_more::From;
use galvan_ast_macro::AstNode;
use typeunion::type_union;

#[type_union]
#[derive(Debug, PartialEq, Eq, AstNode)]
pub type PostfixExpression = YeetExpression + AccessExpression;

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct AccessExpression {
    pub base: Expression,
    pub index: Expression,
    pub span: Span,
}

#[derive(Debug, From, PartialEq, Eq, AstNode)]
pub struct YeetExpression {
    pub inner: Expression,
    pub span: Span,
}
