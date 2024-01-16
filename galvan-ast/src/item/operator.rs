use super::*;

use derive_more::From;
use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::operator_chain))]
pub struct OperatorChain {
    pub parts: Vec<SimpleExpression>,
    pub operators: Vec<InfixOperator>,
}

#[derive(Debug, Clone, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::infix_operator))]
pub enum InfixOperator {
    Arithmetic(ArithmeticOperator),
    // Bitwise(BitwiseOperator),
    Collection(CollectionOperator),
    Comparison(ComparisonOperator),
    Logical(LogicalOperator),
    CustomInfix(CustomInfixOperator),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOperator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Remainder,
    Power,
}

impl FromPest<'_> for ArithmeticOperator {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(NoMatch)?;
        if pair.as_rule() != Rule::arithmetic_operator {
            return Err(NoMatch);
        }
        let pair = pair.into_inner().next().ok_or(NoMatch)?;
        match pair.as_rule() {
            Rule::plus => Ok(ArithmeticOperator::Plus),
            Rule::minus => Ok(ArithmeticOperator::Minus),
            Rule::multiply => Ok(ArithmeticOperator::Multiply),
            Rule::divide => Ok(ArithmeticOperator::Divide),
            Rule::remainder => Ok(ArithmeticOperator::Remainder),
            Rule::power => Ok(ArithmeticOperator::Power),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionOperator {
    Concat,
    Remove,
    Contains,
}

impl FromPest<'_> for CollectionOperator {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(NoMatch)?;
        if pair.as_rule() != Rule::collection_operator {
            return Err(NoMatch);
        }
        let pair = pair.into_inner().next().ok_or(NoMatch)?;
        match pair.as_rule() {
            Rule::concat => Ok(CollectionOperator::Concat),
            Rule::remove => Ok(CollectionOperator::Remove),
            Rule::contains => Ok(CollectionOperator::Contains),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Identical,
    NotIdentical,
}

impl FromPest<'_> for ComparisonOperator {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(NoMatch)?;
        if pair.as_rule() != Rule::comparison_operator {
            return Err(NoMatch);
        }
        let pair = pair.into_inner().next().ok_or(NoMatch)?;
        match pair.as_rule() {
            Rule::equal => Ok(ComparisonOperator::Equal),
            Rule::not_equal => Ok(ComparisonOperator::NotEqual),
            Rule::less => Ok(ComparisonOperator::Less),
            Rule::less_equal => Ok(ComparisonOperator::LessEqual),
            Rule::greater => Ok(ComparisonOperator::Greater),
            Rule::greater_equal => Ok(ComparisonOperator::GreaterEqual),
            Rule::identical => Ok(ComparisonOperator::Identical),
            Rule::not_identical => Ok(ComparisonOperator::NotIdentical),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOperator {
    And,
    Or,
    Xor,
}

impl FromPest<'_> for LogicalOperator {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(NoMatch)?;
        if pair.as_rule() != Rule::logical_infix_operator {
            return Err(NoMatch);
        }
        let pair = pair.into_inner().next().ok_or(NoMatch)?;
        match pair.as_rule() {
            Rule::and => Ok(LogicalOperator::And),
            Rule::or => Ok(LogicalOperator::Or),
            Rule::xor => Ok(LogicalOperator::Xor),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::custom_infix_operator))]
pub struct CustomInfixOperator(#[pest_ast(outer(with(string)))] String);
