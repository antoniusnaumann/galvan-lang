use derive_more::{Display, From};

use crate::AstNode;

#[derive(Clone, Debug, Display, PartialEq, Eq, From, Hash)]
pub struct Ident(String);

impl Ident {
    pub fn new(name: impl Into<String>) -> Ident {
        Ident(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AstNode for Ident {
    fn span(&self) -> &crate::Span {
        todo!()
    }

    fn print(&self, indent: usize) -> String {
        format!("{}{}", " ".repeat(indent), self.0)
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, Hash, From)]
pub struct TypeIdent(String);

impl TypeIdent {
    pub fn new(name: impl Into<String>) -> TypeIdent {
        TypeIdent(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AstNode for TypeIdent {
    fn span(&self) -> &crate::Span {
        todo!()
    }

    fn print(&self, indent: usize) -> String {
        self.0.clone()
    }
}

impl AsRef<str> for TypeIdent {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
