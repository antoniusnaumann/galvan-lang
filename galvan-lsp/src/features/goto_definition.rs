//! `textDocument/definition`.

use tower_lsp::lsp_types::{Location, Position, Url};

use crate::analysis;
use crate::document::Document;

/// Resolve the declaration of the symbol at `position`.
///
/// Definitions are currently looked up within the same document only;
/// cross-file resolution is blocked on the resolver exposing source spans for
/// imported items (see `compiler-features.md`).
pub fn goto_definition(document: &Document, uri: Url, position: Position) -> Option<Location> {
    let tree = document.tree.as_ref()?;
    let text = document.text();
    let offset = document.line_index.offset(text, position)?;

    let symbol = analysis::symbol_at(tree, text, offset)?;
    let lookup = analysis::lookup_context(document)?;
    let resolved = analysis::resolve(&lookup, &symbol)?;

    Some(Location {
        uri,
        range: document.line_index.range(text, resolved.span),
    })
}
