use derive_more::{Display, From};

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

impl AsRef<str> for TypeIdent {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
