//! `textDocument/completion`.
//!
//! Offers top-level functions and types declared anywhere in the crate plus a
//! fixed set of language keywords. Scope-aware completion (locals, members,
//! imported symbols) is blocked on the compiler exposing position-indexed scope
//! information — see `compiler-features.md`.

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind};

use crate::analysis::type_decl_span;
use crate::features::span_text;
use crate::workspace::Crate;

/// Galvan keywords offered unconditionally.
const KEYWORDS: &[&str] = &[
    "fn", "type", "test", "main", "let", "mut", "ref", "if", "else", "while", "for", "in",
    "return", "match", "true", "false", "and", "or", "not", "use", "pub", "struct", "enum",
    "break", "continue", "async", "build", "cmd",
];

pub fn completion(krate: &Crate) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for file in krate.files() {
        let Some(segmented) = file.segmented.as_ref() else {
            continue;
        };
        let text = file.source.content();

        for func in &segmented.functions {
            items.push(CompletionItem {
                label: func.signature.identifier.as_str().to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(span_text(text, func.signature.span).to_string()),
                ..Default::default()
            });
        }

        for ty in &segmented.types {
            let header = span_text(text, type_decl_span(ty));
            let header = header.split('{').next().unwrap_or(header).trim();
            items.push(CompletionItem {
                label: ty.ident().as_str().to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some(header.to_string()),
                ..Default::default()
            });
        }
    }

    for keyword in KEYWORDS {
        items.push(CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }

    items
}
