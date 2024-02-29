use typeunion::type_union;

use super::*;

#[derive(Debug, PartialEq, Eq)]
pub struct Assignment {
    pub target: AssignmentTarget,
    pub operator: AssignmentOperator,
    pub expression: Expression,
}

#[type_union]
#[derive(Debug, PartialEq, Eq)]
pub type AssignmentTarget = Ident + MemberChain;

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
