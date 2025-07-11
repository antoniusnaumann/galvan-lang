use crate::{AstNode, DeclModifier, Expression, Ident, PrintAst, Span, TypeIdent};
use derive_more::From;
use galvan_ast_macro::AstNode;

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct FunctionCall {
    pub identifier: Ident,
    pub arguments: Vec<FunctionCallArg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct FunctionCallArg {
    pub modifier: Option<DeclModifier>,
    pub expression: Expression,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct ConstructorCall {
    pub identifier: TypeIdent,
    pub arguments: Vec<ConstructorCallArg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, From, AstNode)]
pub struct ConstructorCallArg {
    pub ident: Ident,
    pub expression: Expression,
    pub span: Span,
}
