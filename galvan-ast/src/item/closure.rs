use crate::{Block, Expression, Ident, TypeElement};

#[derive(Debug, PartialEq, Eq)]
pub struct Closure {
    pub arguments: Vec<ClosureArgument>,
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureArgument {
    pub ident: Ident,
    pub ty: Option<TypeElement>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ElseExpression {
    pub receiver: Box<Expression>,
    pub block: Block,
}
