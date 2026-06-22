use derive_more::From;
use galvan_ast_macro::AstNode;
use typeunion::type_union;

use super::Expression;
use crate::{AstNode, DeclModifier, PrintAst, Span};

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

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct ModifiedExpression {
    pub inner: Expression,
    pub modifier: DeclModifier,
    pub span: Span,
}
