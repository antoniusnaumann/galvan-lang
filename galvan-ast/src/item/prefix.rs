use galvan_ast_macro::PrintAst;

use crate::{Expression, PrintAst};

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct RefExpression {
    pub inner: Box<Expression>,
}
