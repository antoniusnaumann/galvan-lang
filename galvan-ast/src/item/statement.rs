use galvan_ast_macro::AstNode;
use typeunion::type_union;

use super::*;
use crate::item::closure::Closure;
use crate::{AstNode, Span};

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct Body {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    pub body: Body,
}

#[type_union]
#[derive(Debug, PartialEq, Eq, AstNode)]
pub type Statement = Assignment + Declaration + Expression; // + Block;

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct Declaration {
    pub decl_modifier: DeclModifier,
    pub identifier: Ident,
    pub type_annotation: Option<TypeElement>,
    pub assignment: Option<Expression>,
    pub span: Span,
}

type Infix = Box<InfixExpression>;
type Postfix = Box<PostfixExpression>;

#[type_union]
#[derive(Debug, PartialEq, Eq, AstNode)]
pub type Expression =
    ElseExpression 
    + FunctionCall
    + Infix 
    + Postfix
    + CollectionLiteral
    + ConstructorCall
    + Literal
    + Ident
    + Closure
    + Group;

#[derive(Debug, PartialEq, Eq)]
pub struct Group {
    pub inner: Box<Expression>,
    pub span: Span,
}
