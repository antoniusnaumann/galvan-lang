use galvan_ast_macro::AstNode;

use crate::{Block, Expression, Ident, Span, TypeElement, AstNode};

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct Closure {
    pub arguments: Vec<ClosureArgument>,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, AstNode)]
pub struct ClosureArgument {
    pub ident: Ident,
    pub ty: Option<TypeElement>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct ElseExpression {
    pub receiver: Box<Expression>,
    pub block: Block,
    pub span: Span,
}
