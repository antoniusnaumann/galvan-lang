use galvan_ast_macro::{AstNode, PrintAst};
use typeunion::type_union;

use super::*;
use crate::item::closure::Closure;
use crate::{AstNode, PrintAst, Span};

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct Body {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct Block {
    pub body: Body,
    pub span: Span,
}

#[type_union]
#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub type Statement = Assignment + Declaration + Expression + Return + Throw; // + Block;

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct Declaration {
    pub decl_modifier: DeclModifier,
    pub identifier: Ident,
    pub type_annotation: Option<TypeElement>,
    pub assignment: Option<Expression>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct Return {
    pub expression: Expression,
    pub is_explicit: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct Throw {
    pub expression: Expression,
    pub span: Span,
}

type Infix = Box<InfixExpression>;
type Postfix = Box<PostfixExpression>;

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct Group {
    pub inner: Box<Expression>,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub span: Span,
}

#[type_union]
#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub type ExpressionKind = ElseExpression
    + FunctionCall
    + Infix
    + Postfix
    + CollectionLiteral
    + ConstructorCall
    + EnumAccess
    + Literal
    + Ident
    + Closure
    + Group;

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct EnumAccess {
    pub target: TypeIdent,
    pub case: TypeIdent,
    pub span: Span,
}
