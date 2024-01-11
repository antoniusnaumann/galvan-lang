use crate::Expression;
use galvan_pest::Rule;
use typeunion::type_union;

#[type_union]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::collection_literal))]
pub type CollectionLiteral = ArrayLiteral + DictLiteral + SetLiteral + OrderedDictLiteral;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::array_literal))]
pub struct ArrayLiteral {
    pub elements: Vec<Expression>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::dict_literal))]
pub struct DictLiteral {
    pub elements: Vec<DictLiteralElement>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::dict_literal_element))]
pub struct DictLiteralElement {
    pub key: Expression,
    pub value: Expression,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::set_literal))]
pub struct SetLiteral {
    pub elements: Vec<Expression>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::ordered_dict_literal))]
pub struct OrderedDictLiteral {
    pub elements: Vec<DictLiteralElement>,
}
