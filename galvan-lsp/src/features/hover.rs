//! `textDocument/hover`.

use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

use crate::analysis::{self, ResolvedKind};
use crate::document::Document;
use crate::features::{doc_comment, span_text};
use crate::workspace::Crate;

pub fn hover(current: &Document, krate: &Crate, position: Position) -> Option<Hover> {
    let tree = current.tree.as_ref()?;
    let offset = current.line_index.offset(&current.text, position)?;
    let token = analysis::symbol_at(tree, &current.text, offset)?;

    let lookup = krate.lookup();
    let resolved = analysis::resolve(&lookup, &token.symbol)?;

    // The declaration may live in another file, so format from its own source.
    let target = resolved.source.content();
    let signature = match resolved.kind {
        ResolvedKind::Function(func) => span_text(target, func.signature.span),
        ResolvedKind::Type(decl) => {
            // Show the type header, not the entire body which may be large.
            let full = span_text(target, analysis::type_decl_span(decl));
            full.split('{').next().unwrap_or(full).trim()
        }
    };

    let mut markdown = format!("```galvan\n{signature}\n```");
    if let Some(doc) = doc_comment(target, resolved.span) {
        markdown.push_str("\n\n");
        markdown.push_str(&doc);
    }

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        // Highlight the hovered token in the current document.
        range: Some(
            current
                .line_index
                .byte_range(&current.text, token.range.0, token.range.1),
        ),
    })
}
