use crate::{AstNode, PrintAst, Span};
use derive_more::From;
use galvan_ast_macro::AstNode;
use typeunion::type_union;

use super::Expression;

#[type_union]
#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub type PostfixExpression = YeetExpression + AccessExpression;

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct AccessExpression {
    pub base: Expression,
    pub index: Expression,
    pub span: Span,
}

#[derive(Clone, Debug, From, PartialEq, Eq, AstNode)]
pub struct YeetExpression {
    pub inner: Expression,
    pub span: Span,
}
