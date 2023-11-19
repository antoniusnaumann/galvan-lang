use derive_more::From;

use super::*;

#[derive(Debug, From, FromPest)]
#[pest_ast(rule(Rule::string_literal))]
pub struct StringLiteral(#[pest_ast(outer(with(string)))] String);