use galvan_ast_macro::{AstNode, ast_node};
use typeunion::type_union;

use super::*;
use crate::item::closure::Closure;
use crate::{AstNode, Span};


#[derive(Debug, PartialEq, Eq, AstNode)]
#[ast_node]
pub struct Body {
    pub statements: Vec<Statement>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    pub body: Body,
}

#[type_union]
#[derive(Debug, PartialEq, Eq)]
pub type Statement = Assignment + Declaration + Expression; // + Block;

#[derive(Debug, PartialEq, Eq)]
pub struct Declaration {
    pub decl_modifier: DeclModifier,
    pub identifier: Ident,
    pub type_annotation: Option<TypeElement>,
    pub assignment: Option<Expression>,
}

type Infix = Box<InfixExpression>;
type Postfix = Box<PostfixExpression>;

#[type_union]
#[derive(Debug, PartialEq, Eq)]
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
pub struct Group(pub Box<Expression>);
