use derive_more::From;
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
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
