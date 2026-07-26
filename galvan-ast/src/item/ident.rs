use std::fmt;
use std::hash::{Hash, Hasher};

use crate::{AstNode, Span};

/// A value-level identifier (variable, function or field name).
///
/// Carries the source span of the identifier token. The span is *not* part of
/// the identifier's identity: equality and hashing only consider the name, so
/// idents can be used as lookup keys regardless of where they were written.
#[derive(Clone, Debug)]
pub struct Ident {
    name: String,
    span: Span,
}

impl Ident {
    pub fn new(name: impl Into<String>) -> Ident {
        Ident {
            name: name.into(),
            span: Span::default(),
        }
    }

    /// An identifier as written at a specific location in the source.
    pub fn spanned(name: impl Into<String>, span: Span) -> Ident {
        Ident {
            name: name.into(),
            span,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }

    pub fn is_self(&self) -> bool {
        self.name == "self"
    }
}

impl PartialEq for Ident {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Ident {}

impl Hash for Ident {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<String> for Ident {
    fn from(name: String) -> Self {
        Ident::new(name)
    }
}

impl AstNode for Ident {
    fn span(&self) -> Span {
        self.span
    }

    fn print(&self, indent: usize) -> String {
        format!("{}{}", " ".repeat(indent), self.name)
    }
}

/// A type-level identifier.
///
/// Like [`Ident`], the span is not part of the identifier's identity.
#[derive(Clone, Debug)]
pub struct TypeIdent {
    name: String,
    span: Span,
}

impl TypeIdent {
    pub fn new(name: impl Into<String>) -> TypeIdent {
        let name: String = name.into();
        TypeIdent {
            name: name.trim().to_owned(),
            span: Span::default(),
        }
    }

    /// A type identifier as written at a specific location in the source.
    pub fn spanned(name: impl Into<String>, span: Span) -> TypeIdent {
        let name: String = name.into();
        TypeIdent {
            name: name.trim().to_owned(),
            span,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }

    pub fn is_intrinsic(&self) -> bool {
        self.name.starts_with("__")
    }
}

impl PartialEq for TypeIdent {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for TypeIdent {}

impl Hash for TypeIdent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl fmt::Display for TypeIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<TypeIdent> for String {
    fn from(value: TypeIdent) -> Self {
        value.name
    }
}

impl AstNode for TypeIdent {
    fn span(&self) -> Span {
        self.span
    }

    fn print(&self, _indent: usize) -> String {
        self.name.clone()
    }
}

impl AsRef<str> for TypeIdent {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
