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
            .unwrap_or_else(|| type_id.to_string().into())
    }

    pub(crate) fn get_borrowed(&self, type_id: &TypeIdent) -> Cow<str> {
        self.types
            .get(type_id)
            .map(RustType::borrowed)
            .map(Cow::Borrowed)
            .unwrap_or_else(|| type_id.to_string().into())
    }

    pub(crate) fn get_mut_borrowed(&self, type_id: &TypeIdent) -> Cow<str> {
        self.types
            .get(type_id)
            .map(RustType::mut_borrowed)
            .map(Cow::Borrowed)
            .unwrap_or_else(|| type_id.to_string().into())
    }

    pub(crate) fn is_copy(&self, type_id: &TypeIdent) -> bool {
        self.types
            .get(type_id)
            .map(|rust_type| rust_type.is_copy)
            .unwrap_or(false)
    }
}

macro_rules! mapping {
    ($($tokens:tt),* $(,)?) => {
        {
            use crate::mapping::{Mapping, RustType};
            let types = ::std::collections::HashMap::new();
            let mut mapping = Mapping { types };
            $(
                crate::mapping::mapping_insert!(mapping, $tokens);
            )*
            mapping
        }
    };
}
macro_rules! mapping_insert {
    ($mapping:ident, ($type_id:literal => $owned:literal, $borrowed:literal, $mut_borrowed:literal, copy)) => {
        $mapping.types.insert(
            ::galvan_ast::TypeIdent::new($type_id),
            RustType::new($owned, $borrowed, $mut_borrowed, true),
        );
    };
    ($mapping:ident, ($type_id:literal => $owned:literal, $borrowed:literal, $mut_borrowed:literal)) => {
        $mapping.types.insert(
            ::galvan_ast::TypeIdent::new($type_id),
            RustType::new($owned, $borrowed, $mut_borrowed, false),
        );
    };
    ($mapping:ident, ($type_id:literal => $owned:literal, $borrowed:literal)) => {
        $mapping.types.insert(
            ::galvan_ast::TypeIdent::new($type_id),
            RustType::new($owned, $borrowed, $owned, false),
        );
    };
    ($mapping:ident, ($type_id:literal => $owned:literal)) => {
        $mapping.types.insert(
            ::galvan_ast::TypeIdent::new($type_id),
            RustType::new($owned, $owned, $owned, false),
        );
    };
    ($mapping:ident, ($type_id:literal => $owned:literal, copy)) => {
        $mapping.types.insert(
            ::galvan_ast::TypeIdent::new($type_id),
            RustType::new($owned, $owned, $owned, true),
        );
    };
}

pub(crate) use mapping;
pub(crate) use mapping_insert;

/// Transpiled type names, depending on whether they are owned, borrowed, or mutably borrowed.
#[derive(Debug, Clone)]
pub(crate) struct RustType {
    owned: Box<str>,
    borrowed: Box<str>,
    mut_borrowed: Box<str>,
    is_copy: bool,
}

impl RustType {
    pub fn new(
        owned: impl Into<Box<str>>,
        borrowed: impl Into<Box<str>>,
        mut_borrowed: impl Into<Box<str>>,
        is_copy: bool,
    ) -> Self {
        Self {
            owned: owned.into(),
            borrowed: borrowed.into(),
            mut_borrowed: mut_borrowed.into(),
            is_copy,
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
