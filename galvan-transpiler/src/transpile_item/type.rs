use crate::{Transpile, transpile, TypeElement};

impl Transpile for TypeElement {
    fn transpile(self) -> String {
        transpile_type_item(self)
    }
}

fn transpile_type_item(item: TypeElement) -> String {
    match item {
        // TODO: Optimization: Use
        // TODO: Feature: Add a feature flag that allows enabling small vec here
        TypeElement::Array(array) => transpile!("std::collections::Vec<{}>", array.elements),
        TypeElement::Dictionary(dict) => {
            transpile!("std::collections::HashMap<{}, {}>", dict.key, dict.value)
        }
        TypeElement::OrderedDictionary(ordered_dict) => todo!("Use indexmap crate"),
        TypeElement::Set(set) => transpile!("std::collections::HashSet<{}>", set.elements),
        TypeElement::Tuple(tuple) => transpile!("({})", tuple.elements),
        TypeElement::Optional(optional) => transpile!("Option<{}>", optional.some),
        TypeElement::Result(result) => todo!("Use anyhow result"),
        TypeElement::Plain(plain) => format!("{}", plain.ident),
    }
}
