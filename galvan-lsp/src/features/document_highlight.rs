//! `textDocument/documentHighlight`.
//!
//! Highlights every occurrence of the symbol under the cursor within the
//! current file: the same spans `references` reports, restricted to one
//! file and returned as ranges instead of locations. The defining
//! occurrence is marked as a write, uses as reads.

use std::path::Path;

use tower_lsp::lsp_types::{DocumentHighlight, DocumentHighlightKind, Position};

use crate::analysis;
use crate::document::Document;
use crate::workspace::Crate;

pub fn document_highlight(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
) -> Vec<DocumentHighlight> {
    let Some(offset) = current.line_index.offset(&current.text, position) else {
        return Vec::new();
    };
    let Some(analysis) = krate.analyze() else {
        return Vec::new();
    };
    let Some(file) = file else {
        return Vec::new();
    };
    let Some((id, definition)) = analysis::symbol_at(&analysis.index, file, offset) else {
        return Vec::new();
    };

    let mut highlights = Vec::new();
    if let Some(site) = analysis::definition_site(definition) {
        if site.source.origin() == Some(file) {
            highlights.push(highlight(
                current,
                site.target.range.0,
                site.target.range.1,
                DocumentHighlightKind::WRITE,
            ));
        }
    }
    for reference in analysis.index.references(id) {
        if reference.source.origin() == Some(file) {
            highlights.push(highlight(
                current,
                reference.span.range.0,
                reference.span.range.1,
                DocumentHighlightKind::READ,
            ));
        }
    }
    highlights
}

fn highlight(
    current: &Document,
    start: usize,
    end: usize,
    kind: DocumentHighlightKind,
) -> DocumentHighlight {
    DocumentHighlight {
        range: current.line_index.byte_range(&current.text, start, end),
        kind: Some(kind),
    }
}
