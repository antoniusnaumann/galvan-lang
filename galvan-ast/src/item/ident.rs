use derive_more::{Display, From};

use crate::{AstNode, Span};

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
    fn span(&self) -> Span {
        // TODO  Save a meaningful span in this struct
        Span::default()
    }

    fn print(&self, indent: usize) -> String {
        format!("{}{}", " ".repeat(indent), self.0)
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, Hash)]
pub struct TypeIdent(String);

impl TypeIdent {
    pub fn new(name: impl Into<String>) -> TypeIdent {
        let name: String = name.into();
        TypeIdent(name.trim().to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<TypeIdent> for String {
    fn from(value: TypeIdent) -> Self {
        value.0
    }
}

impl AstNode for TypeIdent {
    fn span(&self) -> Span {
        // TODO  Save a meaningful span in this struct
        Span::default()

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
