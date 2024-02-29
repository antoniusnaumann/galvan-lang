use derive_more::{Display, From};
use typeunion::type_union;

#[type_union]
#[derive(Clone, Debug, PartialEq, Eq)]
pub type Literal = StringLiteral + NumberLiteral + BooleanLiteral + NoneLiteral;

#[derive(Clone, Debug, PartialEq, Eq, From)]
pub struct StringLiteral(String);

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

#[derive(Clone, Debug, PartialEq, Eq, From)]
// TODO: Parse number literal and validate type
pub struct NumberLiteral(String);

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

#[derive(Clone, Copy, Display, Debug, PartialEq, Eq, From)]
pub struct NoneLiteral;

