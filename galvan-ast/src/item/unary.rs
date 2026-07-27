use galvan_ast_macro::AstNode;

use crate::{AstNode, Expression, PrintAst, Span};

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct UnaryExpression {
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    LogicalNot,
}

impl PrintAst for UnaryOperator {
    fn print_ast(&self, indent: usize) -> String {
        format!("{}{}", " ".repeat(indent), self.symbol())
    }
}

impl UnaryOperator {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::LogicalNot => "not",
        }
    }
}
