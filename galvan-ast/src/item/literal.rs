use derive_more::From;
use galvan_pest::Rule;

use super::string;

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::string_literal))]
pub struct StringLiteral(#[pest_ast(outer(with(string)))] String);