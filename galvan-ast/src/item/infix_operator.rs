use derive_more::From;

use crate::Expression;

pub trait InfixOperator {}

#[derive(Clone, Debug, PartialEq, Eq, From)]
pub struct InfixExpression<Op: InfixOperator> {
    pub lhs: Expression,
    pub operator: Op,
    pub rhs: Expression,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicalOperator {
    Or,
    And,
    Xor,
}

impl InfixOperator for LogicalOperator {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArithmeticOperator {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Exp,
}

impl InfixOperator for ArithmeticOperator {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollectionOperator {
    Concat,
    Remove,
    Contains,
}

impl InfixOperator for CollectionOperator {}


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
    LessEqual,
    Less,
    GreaterEqual,
    Greater,
    Equal,
    NotEqual,
    Identical,
    NotIdentical,
}

impl InfixOperator for ComparisonOperator {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberOperator {
    Dot,
    SafeCall,
}

impl InfixOperator for MemberOperator {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomInfix(String); 
