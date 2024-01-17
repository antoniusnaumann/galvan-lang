use super::*;

use derive_more::From;
use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;
use typeunion::type_union;

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

#[derive(Debug, PartialEq, Eq)]
pub struct OperatorTree {
    pub left: OperatorTreeNode,
    pub operator: InfixOperator,
    pub right: OperatorTreeNode,
}

pub type Operation = Box<OperatorTree>;
#[type_union]
#[derive(Debug, PartialEq, Eq)]
pub type OperatorTreeNode = Operation + SimpleExpression;

trait Precedence {
    fn precedence(&self) -> u8;
}

impl Precedence for LogicalOperator {
    fn precedence(&self) -> u8 {
        match self {
            LogicalOperator::And => 8,
            LogicalOperator::Or => 5,
            LogicalOperator::Xor => 2,
        }
    }
}

impl Precedence for ComparisonOperator {
    fn precedence(&self) -> u8 {
        match self {
            ComparisonOperator::Equal
            | ComparisonOperator::NotEqual
            | ComparisonOperator::Less
            | ComparisonOperator::LessEqual
            | ComparisonOperator::Greater
            | ComparisonOperator::GreaterEqual => 12,
            ComparisonOperator::Identical | ComparisonOperator::NotIdentical => 15,
        }
    }
}

impl Precedence for CollectionOperator {
    fn precedence(&self) -> u8 {
        match self {
            CollectionOperator::Concat | CollectionOperator::Remove => 22,
            CollectionOperator::Contains => 25,
        }
    }
}

impl Precedence for ArithmeticOperator {
    fn precedence(&self) -> u8 {
        match self {
            ArithmeticOperator::Plus | ArithmeticOperator::Minus => 32,
            ArithmeticOperator::Multiply
            | ArithmeticOperator::Divide
            | ArithmeticOperator::Remainder => 35,
            ArithmeticOperator::Power => 38,
        }
    }
}

impl Precedence for CustomInfixOperator {
    fn precedence(&self) -> u8 {
        50
    }
}

impl OperatorChain {
    pub fn into_tree(self) -> OperatorTree {
        todo!("Implement operator chain to tree conversion")
    }
}
