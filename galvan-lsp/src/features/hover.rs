//! `textDocument/hover`.

use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

use crate::analysis::{self, ResolvedKind};
use crate::document::Document;
use crate::features::{doc_comment, span_text};

pub fn hover(document: &Document, position: Position) -> Option<Hover> {
    let tree = document.tree.as_ref()?;
    let text = document.text();
    let offset = document.line_index.offset(text, position)?;

    let symbol = analysis::symbol_at(tree, text, offset)?;
    let lookup = analysis::lookup_context(document)?;
    let resolved = analysis::resolve(&lookup, &symbol)?;

    let (signature, target_span) = match resolved.kind {
        ResolvedKind::Function(func) => (span_text(text, func.signature.span), func.signature.span),
        ResolvedKind::Type(decl) => {
            let span = analysis::type_decl_span(decl);
            // Show the type header, not the entire body which may be large.
            let full = span_text(text, span);
            let header = full.split('{').next().unwrap_or(full).trim();
            (header, span)
        }
    };

    let mut markdown = format!("```galvan\n{signature}\n```");
    if let Some(doc) = doc_comment(text, target_span) {
        markdown.push_str("\n\n");
        markdown.push_str(&doc);
    }

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(document.line_index.range(text, resolved.span)),
    })
}
