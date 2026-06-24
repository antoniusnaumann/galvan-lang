use galvan_ast_macro::PrintAst;

use crate::{Block, Expression, Ident, PrintAst, Span, TypeIdent};

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct MatchExpression {
    pub scrutinee: Box<Expression>,
    pub arms: Vec<MatchArm>,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub enum MatchPattern {
    Wildcard(MatchWildcardPattern),
    EnumVariant(MatchEnumPattern),
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct MatchWildcardPattern {
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct MatchEnumPattern {
    pub case: TypeIdent,
    pub arguments: Vec<MatchPatternArg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub enum MatchPatternArg {
    Binding(MatchBindingPattern),
    Named(MatchNamedPatternArg),
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct MatchNamedPatternArg {
    pub field: Ident,
    pub binding: MatchBindingPattern,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub enum MatchBindingPattern {
    Ident(Ident),
    Wildcard(MatchWildcardPattern),
}
