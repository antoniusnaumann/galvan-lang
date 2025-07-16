use galvan_ast_macro::PrintAst;

use crate::{Block, Expression, Ident, PrintAst, TypeElement};

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct Closure {
    pub parameters: Vec<ClosureParameter>,
    pub block: Block,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct ClosureParameter {
    pub ident: Ident,
    pub ty: Option<TypeElement>,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct ElseExpression {
    pub receiver: Box<Expression>,
    pub parameters: Vec<ClosureParameter>,
    pub block: Block,
}
