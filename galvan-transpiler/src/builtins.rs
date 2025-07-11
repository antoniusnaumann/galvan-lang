use galvan_ast::{
    ArrayTypeItem, BasicTypeItem, DictionaryTypeItem, GenericTypeItem, OptionalTypeItem,
    OrderedDictionaryTypeItem, ResultTypeItem, SetTypeItem, TupleTypeItem, TypeElement,
};
use itertools::Itertools;

use crate::mapping::{mapping, Mapping};

/// Mapping of standard Galvan types to Rust types
pub fn builtins() -> Mapping {
    mapping!(
        ("Bool" => "bool", copy),

        // Hack to explicitly enable rusts inference for number literals
        ("__Number" => "_", copy),
        // Symbolic type name for partial inference
        ("__Infer" => "_"),

        ("I8" => "i8", copy),
        ("I16" => "i16", copy),
        ("I32" => "i32", copy),
        ("I64" => "i64", copy),
        ("I128" => "i128", copy),
        ("Int" => "i64", copy),
        ("ISize" => "isize", copy),

        ("U8" => "u8", copy),
        ("U16" => "u16", copy),
        ("U32" => "u32", copy),
        ("U64" => "u64", copy),
        ("U128" => "u128", copy),
        ("UInt" => "u64", copy),
        ("USize" => "usize", copy),

        ("Float" => "f32", copy),
        ("Double" => "f64", copy),

        ("String" => "String", "str"),
        ("Char" => "char", copy),
    )
}

/// Lists all iterator functions that have a closure which borrows its argument, leading to a double iterator when called on .iter()
pub(crate) const BORROWED_ITERATOR_FNS: [&str; 12] = [
    "filter",
    "skip_while",
    "take_while",
    "inspect",
    "partition",
    "find",
    "try_find",
    "max_by_key",
    "max_by",
    "min_by_key",
    "min_by",
    "is_sorted_by",
];

pub trait CheckBuiltins {
    fn is_infer(&self) -> bool;
}

pub trait IsSame {
    fn is_same(&self, other: &Self) -> bool;
}

impl CheckBuiltins for TypeElement {
    fn is_infer(&self) -> bool {
        let TypeElement::Plain(plain) = self else {
            return false;
        };

        plain.ident.as_str() == "__Infer"
    }
}

impl IsSame for TypeElement {
    fn is_same(&self, other: &TypeElement) -> bool {
        match (self, other) {
            (TypeElement::Array(a), TypeElement::Array(b)) => a.is_same(b),
            (TypeElement::Dictionary(a), TypeElement::Dictionary(b)) => a.is_same(b),
            (TypeElement::OrderedDictionary(a), TypeElement::OrderedDictionary(b)) => a.is_same(b),
            (TypeElement::Set(a), TypeElement::Set(b)) => a.is_same(b),
            (TypeElement::Tuple(a), TypeElement::Tuple(b)) => a.is_same(b),
            (TypeElement::Optional(a), TypeElement::Optional(b)) => a.is_same(b),
            (TypeElement::Result(a), TypeElement::Result(b)) => a.is_same(b),
            (TypeElement::Plain(a), TypeElement::Plain(b)) => a.is_same(b),
            (TypeElement::Generic(a), TypeElement::Generic(b)) => a.is_same(b),
            (TypeElement::Never(_), TypeElement::Never(_)) => true,
            _ => false,
        }
    }
}

impl IsSame for ArrayTypeItem {
    fn is_same(&self, other: &Self) -> bool {
        self.elements.is_same(&other.elements)
    }
}

impl IsSame for DictionaryTypeItem {
    fn is_same(&self, other: &Self) -> bool {
        self.key.is_same(&other.key) && self.value.is_same(&other.value)
    }
}

impl IsSame for OrderedDictionaryTypeItem {
    fn is_same(&self, other: &Self) -> bool {
        self.key.is_same(&other.key) && self.value.is_same(&other.value)
    }
}

impl IsSame for SetTypeItem {
    fn is_same(&self, other: &Self) -> bool {
        self.elements.is_same(&other.elements)
    }
}

impl IsSame for TupleTypeItem {
    fn is_same(&self, other: &Self) -> bool {
        if self.elements.len() != other.elements.len() {
            return false;
        }
        for (a, b) in self.elements.iter().zip_eq(&other.elements) {
            if !a.is_same(&b) {
                return false;
            }
        }
        true
    }
}

impl IsSame for OptionalTypeItem {
    fn is_same(&self, other: &Self) -> bool {
        self.some.is_same(&other.some)
    }
}

impl IsSame for ResultTypeItem {
    fn is_same(&self, other: &Self) -> bool {
        self.success.is_same(&other.success)
            && ((self.error.is_none() && other.error.is_none())
                || (self
                    .error
                    .as_ref()
                    .is_some_and(|a| other.error.as_ref().is_some_and(|b| a.is_same(b)))))
    }
}

impl IsSame for BasicTypeItem {
    fn is_same(&self, other: &Self) -> bool {
        match (self.ident.as_str(), other.ident.as_str()) {
            (a, b) if a == b => true,
            ("I64", "Int") | ("Int", "I64") => true,
            ("__Infer", _) | (_, "__Infer") => true,
            (_, _) => false,
        }
    }
}

impl IsSame for GenericTypeItem {
    fn is_same(&self, other: &Self) -> bool {
        self.ident == other.ident
    }
}
