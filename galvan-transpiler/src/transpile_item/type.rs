use galvan_ast::*;
use crate::{impl_transpile, impl_transpile_fn, impl_transpile_variants, TypeElement};

impl_transpile!(ArrayTypeItem, "std::collections::Vec<{}>", elements);
impl_transpile!(DictionaryTypeItem, "std::collections::HashMap<{}, {}>", key, value);
impl_transpile!(OrderedDictionaryTypeItem, "TODO {} {}", key, value);
impl_transpile!(SetTypeItem, "std::collections::HashSet<{}>", elements);
impl_transpile!(TupleTypeItem, "({})", elements);
impl_transpile_fn!(OptionalTypeItem, "Option<{}>", element);
impl_transpile!(ResultTypeItem, "TODO use anyhow result",);
impl_transpile!(BasicTypeItem, "{}", ident);


impl_transpile_variants! { TypeElement;
    Plain
    Array
    Dictionary
    OrderedDictionary
    Set
    Tuple
    Optional
    Result
}
