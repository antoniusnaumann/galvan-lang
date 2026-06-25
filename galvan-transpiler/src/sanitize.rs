use std::borrow::Cow;

use galvan_ast::{Ident, UsePath};

pub(crate) fn sanitize_name(name: &str) -> Cow<'_, str> {
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{}", name).into()
    } else {
        name.into()
    }
}

pub(crate) fn mangle_function_name<'a>(
    name: &str,
    labels: impl IntoIterator<Item = &'a Ident>,
) -> String {
    let mut name = sanitize_name(name).into_owned();
    for label in labels {
        name.push_str("__");
        name.push_str(&sanitize_name(label.as_str()));
    }
    name
}

pub(crate) fn sanitize_path(path: &UsePath) -> String {
    path.segments
        .iter()
        .map(|segment| sanitize_name(segment.as_str()).into_owned())
        .collect::<Vec<_>>()
        .join("::")
}

const RUST_KEYWORDS: [&str; 49] = [
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "static", "struct", "super", "trait", "true", "try", "type", "unsafe", "use",
    "where", "while", "async", "await", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield",
];
