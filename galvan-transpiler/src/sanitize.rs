use std::borrow::Cow;

pub(crate) fn sanitize_name(name: &str) -> Cow<str> {
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{}", name).into()
    } else {
        name.into()
    }
}

const RUST_KEYWORDS: [&str; 51] = [
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "try", "type",
    "unsafe", "use", "where", "while", "async", "await", "abstract", "become", "box", "do",
    "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield",
];
