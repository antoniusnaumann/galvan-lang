use super::*;
use derive_more::From;
use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;

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

#[derive(Debug, PartialEq, Eq)]
pub struct LogicalOperation {
    pub base: Box<Expression>,
    pub chain: Vec<(LogicalOperator, Box<Expression>)>,
}

impl FromPest<'_> for LogicalOperation {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(NoMatch)?;
        if pair.as_rule() != Rule::logical_expression {
            return Err(NoMatch);
        }
        let mut pairs = pair.into_inner();

        let base = AllowedInLogical::from_pest(&mut pairs)?;
        let mut chain = vec![];

        while pairs.len() > 0 {
            let operator = LogicalOperator::from_pest(&mut pairs)?;
            let expression = AllowedInLogical::from_pest(&mut pairs)?;
            chain.push((operator, Box::new(expression.into())));
        }

        Ok(LogicalOperation {
            base: Box::new(base.into()),
            chain,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ComparisonOperation {
    pub left: Box<Expression>,
    pub operator: ComparisonOperator,
    pub right: Box<Expression>,
}

impl FromPest<'_> for ComparisonOperation {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(NoMatch)?;
        if pair.as_rule() != Rule::comparison_expression {
            return Err(NoMatch);
        }
        let mut pairs = pair.into_inner();

        let left = AllowedInComparison::from_pest(&mut pairs)?;
        let operator = ComparisonOperator::from_pest(&mut pairs)?;
        let right = AllowedInComparison::from_pest(&mut pairs)?;

        Ok(ComparisonOperation {
            left: Box::new(left.into()),
            operator,
            right: Box::new(right.into()),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CollectionOperation {
    pub left: Box<Expression>,
    pub operator: CollectionOperator,
    pub right: Box<Expression>,
}

impl FromPest<'_> for CollectionOperation {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(NoMatch)?;
        if pair.as_rule() != Rule::collection_expression {
            return Err(NoMatch);
        }
        let mut pairs = pair.into_inner();

        let left = AllowedInCollection::from_pest(&mut pairs)?;
        let operator = CollectionOperator::from_pest(&mut pairs)?;
        let right = AllowedInCollection::from_pest(&mut pairs)?;

        Ok(CollectionOperation {
            left: Box::new(left.into()),
            operator,
            right: Box::new(right.into()),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArithmeticOperation {
    pub base: Box<Expression>,
    pub chain: Vec<(ArithmeticOperator, Box<Expression>)>,
}

impl FromPest<'_> for ArithmeticOperation {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(NoMatch)?;
        if pair.as_rule() != Rule::arithmetic_expression {
            return Err(NoMatch);
        }
        let mut pairs = pair.into_inner();

        let base = AllowedInArithmetic::from_pest(&mut pairs)?;
        let mut chain = vec![];

        while pairs.len() > 0 {
            let operator = ArithmeticOperator::from_pest(&mut pairs)?;
            let expression = AllowedInArithmetic::from_pest(&mut pairs)?;
            chain.push((operator, Box::new(expression.into())));
        }

        Ok(ArithmeticOperation {
            base: Box::new(base.into()),
            chain,
        })
    }
}
