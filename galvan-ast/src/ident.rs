use derive_more::{Display, From};

use super::*;

#[derive(Debug, Display, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::ident))]
pub struct Ident(#[pest_ast(outer(with(string)))] String);

impl Ident {
    pub fn new(name: impl Into<String>) -> Ident {
        Ident(name.into())
    }
}
