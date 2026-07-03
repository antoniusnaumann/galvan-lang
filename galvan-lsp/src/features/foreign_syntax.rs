//! Recognition of keywords from other popular languages.
//!
//! Newcomers write `func`, `switch`, `class`, … out of habit. Depending on
//! the token, that either breaks the parse (`func greet()` produces a
//! tree-sitter error node) or parses fine and only fails typechecking as an
//! unknown identifier (`switch x { … }` parses as a trailing-closure call,
//! `null` as a plain identifier). Both layers funnel into the table here, so
//! [`diagnostics`](crate::features::diagnostics) can explain the mistake and
//! [`code_actions`](crate::features::code_actions) can offer the Galvan
//! equivalent as a quickfix.

use std::path::Path;

use crate::document::Document;
use crate::features::is_ident_byte;
use crate::workspace::Crate;

/// Well-known keywords from other mainstream languages and the Galvan
/// construct they map to.
///
/// `&&` and `||` are deliberately absent: the grammar already accepts them
/// as spellings of `and`/`or`.
const FOREIGN_KEYWORDS: &[(&str, &str)] = &[
    ("function", "fn"),
    ("func", "fn"),
    ("def", "fn"),
    ("fun", "fn"),
    ("switch", "match"),
    ("class", "type"),
    ("struct", "type"),
    ("interface", "type"),
    ("trait", "type"),
    ("enum", "type"),
    ("var", "mut"),
    ("elif", "else if"),
    ("elsif", "else if"),
    ("null", "none"),
    ("nil", "none"),
    ("None", "none"),
    ("import", "use"),
    ("require", "use"),
    ("include", "use"),
    ("True", "true"),
    ("False", "false"),
];

/// The Galvan equivalent of `word`, when `word` is a well-known keyword of
/// another language.
pub fn galvan_equivalent(word: &str) -> Option<&'static str> {
    FOREIGN_KEYWORDS
        .iter()
        .find(|(foreign, _)| *foreign == word)
        .map(|(_, galvan)| *galvan)
}

/// The diagnostic message explaining a foreign keyword.
pub fn message(found: &str, replacement: &str) -> String {
    format!("`{found}` is not a Galvan keyword — Galvan uses `{replacement}`")
}

/// A foreign keyword found in the document.
pub struct ForeignToken {
    /// Byte range of the token in the document text.
    pub start: usize,
    pub end: usize,
    pub found: String,
    pub replacement: &'static str,
}

/// Foreign keywords inside the document's syntax-error regions.
///
/// Only error nodes are scanned: every foreign keyword is a legal Galvan
/// identifier, so flagging them in code that parses would produce false
/// positives (the semantic layer catches those cases through
/// unknown-identifier diagnostics instead). Comments and string literals
/// are lexed even inside error regions and are skipped.
pub fn foreign_tokens(document: &Document) -> Vec<ForeignToken> {
    let Some(tree) = document.tree.as_ref() else {
        return Vec::new();
    };
    if !tree.root_node().has_error() {
        return Vec::new();
    }

    let mut error_ranges = Vec::new();
    let mut skip_ranges = Vec::new();
    let mut cursor = tree.walk();
    let mut recurse = true;
    loop {
        if recurse && cursor.goto_first_child() {
            continue;
        }
        let node = cursor.node();
        if node.is_error() {
            error_ranges.push((node.start_byte(), node.end_byte()));
        }
        if matches!(
            node.kind(),
            "comment" | "string_literal" | "raw_string_literal" | "char_literal"
        ) {
            skip_ranges.push((node.start_byte(), node.end_byte()));
        }
        if cursor.goto_next_sibling() {
            recurse = true;
        } else if cursor.goto_parent() {
            recurse = false;
        } else {
            break;
        }
    }

    let mut tokens = Vec::new();
    for (start, end) in error_ranges {
        for (word_start, word) in words(&document.text, start, end) {
            if skip_ranges
                .iter()
                .any(|&(skip_start, skip_end)| word_start >= skip_start && word_start < skip_end)
            {
                continue;
            }
            if let Some(replacement) = galvan_equivalent(word) {
                tokens.push(ForeignToken {
                    start: word_start,
                    end: word_start + word.len(),
                    found: word.to_string(),
                    replacement,
                });
            }
        }
    }
    tokens
}

/// Foreign keywords used as callees in code that *parses*.
///
/// Statement-position keywords slip past the parser as calls: `switch color
/// { … }` parses as a trailing-closure call and `var x = 1` as a free
/// function call. The typechecker lowers unknown callees without complaint
/// (they may resolve to Rust functions at codegen), so the parse tree is
/// consulted directly: a callee that is a foreign keyword *and* resolved to
/// no definition in the symbol index is reported.
pub fn foreign_callees(document: &Document, krate: &Crate, file: &Path) -> Vec<ForeignToken> {
    let Some(tree) = document.tree.as_ref() else {
        return Vec::new();
    };
    let Some(analysis) = krate.analyze() else {
        return Vec::new();
    };

    let mut tokens = Vec::new();
    let mut cursor = tree.walk();
    let mut recurse = true;
    loop {
        if recurse && cursor.goto_first_child() {
            continue;
        }
        let node = cursor.node();
        if matches!(node.kind(), "trailing_closure_expression" | "free_function") {
            let callee = (0..node.named_child_count())
                .filter_map(|i| node.named_child(i))
                .find(|child| child.kind() == "ident");
            if let Some(callee) = callee {
                let (start, end) = (callee.start_byte(), callee.end_byte());
                let word = document.text.get(start..end).unwrap_or("");
                let resolved = analysis.index.reference_at(file, start).is_some()
                    || analysis.index.definition_at(file, start).is_some();
                if let (Some(replacement), false) = (galvan_equivalent(word), resolved) {
                    tokens.push(ForeignToken {
                        start,
                        end,
                        found: word.to_string(),
                        replacement,
                    });
                }
            }
        }
        if cursor.goto_next_sibling() {
            recurse = true;
        } else if cursor.goto_parent() {
            recurse = false;
        } else {
            break;
        }
    }
    tokens
}

/// Maximal identifier-byte runs in `text[start..end]`, with their offsets.
fn words(text: &str, start: usize, end: usize) -> Vec<(usize, &str)> {
    let bytes = &text.as_bytes()[start.min(text.len())..end.min(text.len())];
    let mut words = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        if is_ident_byte(bytes[offset]) {
            let word_start = offset;
            while offset < bytes.len() && is_ident_byte(bytes[offset]) {
                offset += 1;
            }
            words.push((start + word_start, &text[start + word_start..start + offset]));
        } else {
            offset += 1;
        }
    }
    words
}
