use galvan_ast::TypeIdent;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct Mapping {
    pub(crate) types: HashMap<TypeIdent, RustType>,
}

impl Mapping {
    pub(crate) fn get_owned(&self, type_id: &TypeIdent) -> Cow<str> {
        self.types
            .get(type_id)
            .map(RustType::owned)
            .map(Cow::Borrowed)
            .unwrap_or_else(|| type_id.as_str().to_owned().into())
    }

    pub(crate) fn get_borrowed(&self, type_id: &TypeIdent) -> Cow<str> {
        self.types
            .get(type_id)
            .map(RustType::borrowed)
            .map(Cow::Borrowed)
            .unwrap_or_else(|| format!("&{}", type_id.as_str()).into())
    }

    pub(crate) fn get_mut_borrowed(&self, type_id: &TypeIdent) -> Cow<str> {
        self.types
            .get(type_id)
            .map(RustType::mut_borrowed)
            .map(Cow::Borrowed)
            .unwrap_or_else(|| format!("&mut {}", type_id.as_str()).into())
    }
}

macro_rules! mapping {
    ($($type_id:literal => $owned:literal, $borrowed:literal, $mut_borrowed:literal),* $(,)?) => {
        {
            use crate::mapping::{Mapping, RustType};
            let mut types = ::std::collections::HashMap::new();
            $(types.insert(::galvan_ast::TypeIdent::new($type_id), RustType::new($owned, $borrowed, $mut_borrowed));)*
            Mapping { types }
        }
    };
}
pub(crate) use mapping;

/// Transpiled type names, depending on whether they are owned, borrowed, or mutably borrowed.
#[derive(Debug)]
pub(crate) struct RustType {
    owned: Box<str>,
    borrowed: Box<str>,
    mut_borrowed: Box<str>,
}

impl RustType {
    pub fn new(
        owned: impl Into<Box<str>>,
        borrowed: impl Into<Box<str>>,
        mut_borrowed: impl Into<Box<str>>,
    ) -> Self {
        Self {
            owned: owned.into(),
            borrowed: borrowed.into(),
            mut_borrowed: mut_borrowed.into(),
        }
    }

    fn owned(&self) -> &str {
        &self.owned
    }

    fn borrowed(&self) -> &str {
        &self.borrowed
    }

    fn mut_borrowed(&self) -> &str {
        &self.mut_borrowed
    }
}
