use galvan_parser::TypeItem;

use crate::Transpileable;

macro_rules! transpile {
    ($string:expr, $($items:expr),*) => {
        format!($string, $(transpile_type_item($items)),*)
    };
}

impl Transpileable for TypeItem {
    fn transpile(self) -> String {
        transpile_type_item(self)
    }
}

fn transpile_type_item(item: TypeItem) -> String {
    match item {
        // TODO: Optimization: Use
        // TODO: Feature: Add a feature flag that allows enabling small vec here
        TypeItem::Array(array) => transpile!("std::collections::Vec<{}>", array.elements),
        TypeItem::Dictionary(dict) => {
            transpile!("std::collections::HashMap<{}, {}>", dict.key, dict.value)
        }
        TypeItem::OrderedDictionary(ordered_dict) => todo!("Use indexmap crate"),
        TypeItem::Set(set) => transpile!("std::collections::HashSet<{}>", set.elements),
        TypeItem::Tuple(tuple) => format!(
            "({})",
            tuple
                .elements
                .into_iter()
                .map(|e| transpile_type_item(e))
                .fold("".to_string(), |acc, e| format!("{acc}, {e}"))
        ),
        TypeItem::Optional(optional) => transpile!("Option<{}>", optional.some),
        TypeItem::Result(result) => todo!("Use anyhow result"),
        TypeItem::Plain(plain) => format!("{}", plain.ident),
    }
}
