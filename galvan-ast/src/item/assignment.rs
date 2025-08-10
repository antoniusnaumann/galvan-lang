use galvan_ast_macro::AstNode;

use crate::{AstNode, PrintAst, Span};

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct Assignment {
    pub target: Expression,
    pub operator: AssignmentOperator,
    pub expression: Expression,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssignmentOperator {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    PowAssign,
    ConcatAssign,
}

impl PrintAst for AssignmentOperator {
    fn print_ast(&self, indent: usize) -> String {
        let indent_str = " ".repeat(indent);
        let op = match self {
            AssignmentOperator::Assign => "=",
            AssignmentOperator::AddAssign => "+=",
            AssignmentOperator::SubAssign => "-=",
            AssignmentOperator::MulAssign => "*=",
            AssignmentOperator::DivAssign => "/=",
            AssignmentOperator::RemAssign => "%=",
            AssignmentOperator::PowAssign => "**=",
            AssignmentOperator::ConcatAssign => "++=",
        };

        format!("{indent_str}{op}")
    }
}
