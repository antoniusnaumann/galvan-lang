use crate::Expression;
use derive_more::From;
use typeunion::type_union;

#[type_union]
#[derive(Debug, PartialEq, Eq)]
pub type PostfixExpression = YeetExpression + AccessExpression;

#[derive(Debug, PartialEq, Eq)]
pub struct AccessExpression {
    pub base: Expression,
    pub index: Expression,
}

#[derive(Debug, From, PartialEq, Eq)]
pub struct YeetExpression(pub Expression);
