use crate::Expression;
use typeunion::type_union;

#[type_union]
#[derive(Debug, PartialEq, Eq)]
pub type CollectionLiteral = ArrayLiteral + DictLiteral + SetLiteral + OrderedDictLiteral;

#[derive(Debug, PartialEq, Eq)]
pub struct ArrayLiteral {
    pub elements: Vec<Expression>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DictLiteral {
    pub elements: Vec<DictLiteralElement>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DictLiteralElement {
    pub key: Expression,
    pub value: Expression,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SetLiteral {
    pub elements: Vec<Expression>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrderedDictLiteral {
    pub elements: Vec<DictLiteralElement>,
}
