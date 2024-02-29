use derive_more::From;

use crate::{Expression, Ident};

pub trait InfixOperator {}

#[derive(Debug, PartialEq, Eq, From)]
pub enum InfixExpression {
    Logical(InfixOperation<LogicalOperator>),
    Arithmetic(InfixOperation<ArithmeticOperator>),
    Collection(InfixOperation<CollectionOperator>),
    Comparison(InfixOperation<ComparisonOperator>),
    Member(InfixOperation<MemberOperator>),
    Custom(InfixOperation<CustomInfix>),
}

impl InfixExpression {
    pub fn is_comparison(&self) -> bool {
        matches!(self, Self::Comparison(_))
    }
}

#[derive(Debug, PartialEq, Eq, From)]
pub struct InfixOperation<Op: InfixOperator> {
    pub lhs: Expression,
    pub operator: Op,
    pub rhs: Expression,
}

impl InfixOperation<MemberOperator> {
    pub fn is_field(&self) -> bool {
        match self.rhs {
            Expression::Ident(_) => true,
            // TODO: Expression::Postfix(p) => match self p.without_postfix() {
            // Expression::Ident(_) => true, _ => false },
            _ => false,
        }
    }

    pub fn field_ident(&self) -> Option<&Ident> {
        match &self.rhs {
            Expression::Ident(ident) => Some(ident),
            _ => None,
        }
    }
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

impl InfixOperator for CustomInfix {}
