use super::*;

#[derive(Debug, PartialEq, Eq)]
pub struct Assignment {
    pub target: Expression,
    pub operator: AssignmentOperator,
    pub expression: Expression,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AssignmentOperator {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    PowAssign,
}
