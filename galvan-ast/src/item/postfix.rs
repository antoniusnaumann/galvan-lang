use crate::{Expression, SingleExpression};
use derive_more::From;
use from_pest::pest::iterators::Pairs;
use from_pest::{ConversionError, Void};
use galvan_pest::Rule;
use typeunion::type_union;

#[type_union]
#[derive(Debug, PartialEq, Eq)]
pub type PostfixExpression = YeetExpression + AccessExpression;

pub(crate) fn handle_postfixes(
    base: SingleExpression,
    pairs: &mut Pairs<'_, Rule>,
) -> Result<SingleExpression, ConversionError<Void>> {
    let mut exp = base;
    while let Some(pair) = pairs.next() {
        if pair.as_rule() != Rule::postfix_operator {
            return Err(ConversionError::NoMatch);
        }

        let inner = pair.into_inner().next().unwrap();
        exp = match inner.as_rule() {
            Rule::yeet_operator => {
                SingleExpression::Postfix(PostfixExpression::YeetExpression(exp.into()).into())
            }
            Rule::access_operator => {
                todo!("Implement access operator AST conversion!");
            }
            _ => unreachable!("Unexpected postfix operator rule"),
        }
    }

    Ok(exp)
}

#[derive(Debug, PartialEq, Eq)]
pub struct AccessExpression {
    pub base: SingleExpression,
    pub index: Expression,
}

#[derive(Debug, From, PartialEq, Eq)]
pub struct YeetExpression(pub SingleExpression);
