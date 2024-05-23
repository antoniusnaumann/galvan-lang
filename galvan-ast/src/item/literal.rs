use derive_more::From;
use typeunion::type_union;

use galvan_ast_macro::AstNode;

use crate::{AstNode, Span};

#[type_union]
#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub type Literal = StringLiteral + NumberLiteral + BooleanLiteral + NoneLiteral;

#[derive(Clone, Debug, PartialEq, Eq, From)]
pub struct StringLiteral {
    pub value: String,
    pub span: Span,
}

impl AstNode for StringLiteral {
    fn span(&self) -> &Span {
        &self.span
    }

    fn print(&self, indent: usize) -> String {
        format!("{}\"{}\"", " ".repeat(indent), self.value)
    }
}

impl StringLiteral {
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl From<StringLiteral> for String {
    fn from(string: StringLiteral) -> Self {
        string.value
    }
}

impl AsRef<str> for StringLiteral {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
// TODO: Parse number literal and validate type
pub struct NumberLiteral {
    value: String,
    span: Span,
}

impl AstNode for NumberLiteral {
    fn span(&self) -> &Span {
        &self.span
    }

    fn print(&self, indent: usize) -> String {
        format!("{}{}", " ".repeat(indent), self.value)
    }
}

impl NumberLiteral {
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, From)]
pub struct BooleanLiteral {
    pub value: bool,
    span: Span,
}

impl AstNode for BooleanLiteral {
    fn span(&self) -> &Span {
        &self.span
    }

    fn print(&self, indent: usize) -> String {
        format!("{}{}", " ".repeat(indent), self.value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoneLiteral(Span);

impl AstNode for NoneLiteral {
    fn span(&self) -> &Span {
        &self.0
    }

    fn print(&self, indent: usize) -> String {
        format!("{}None", " ".repeat(indent))
    }
}
