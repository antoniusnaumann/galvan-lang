use derive_more::{Display, From};
use galvan_pest::Rule;

use super::string;

#[derive(Debug, Display, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::ident))]
pub struct Ident(#[pest_ast(outer(with(string)))] String);

impl Ident {
    pub fn new(name: impl Into<String>) -> Ident {
        Ident(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, Hash, From, FromPest)]
#[pest_ast(rule(Rule::type_ident))]
pub struct TypeIdent(#[pest_ast(outer(with(string)))] String);

impl TypeIdent {
    pub fn new(name: impl Into<String>) -> TypeIdent {
        TypeIdent(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for TypeIdent {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
