use derive_more::{Display, From};
use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;

use super::string;

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::string_literal))]
pub struct StringLiteral(#[pest_ast(outer(with(string)))] String);

impl StringLiteral {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<StringLiteral> for String {
    fn from(string: StringLiteral) -> Self {
        string.0
    }
}

impl AsRef<str> for StringLiteral {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::number_literal))]
// TODO: Parse number literal and validate type
pub struct NumberLiteral(#[pest_ast(outer(with(string)))] String);

impl NumberLiteral {
    pub fn new(value: &str) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Display, Debug, PartialEq, Eq, From)]
pub struct BooleanLiteral(pub bool);

impl FromPest<'_> for BooleanLiteral {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let Some(pair) = pairs.next() else {
            return Err(NoMatch);
        };

        if pair.as_rule() != Rule::boolean_literal {
            return Err(NoMatch);
        }

        let mut pairs = pair.into_inner();
        match pairs.next() {
            Some(b) if b.as_rule() == Rule::true_keyword => Ok(BooleanLiteral(true)),
            Some(b) if b.as_rule() == Rule::false_keyword => Ok(BooleanLiteral(false)),
            _ => unreachable!(),
        }
    }
}
