//! `textDocument/definition`.

use tower_lsp::lsp_types::{Location, Position, Url};

use crate::analysis;
use crate::document::Document;
use crate::position::LineIndex;
use crate::workspace::Crate;

/// Resolve the declaration of the symbol at `position`.
///
/// Resolution spans every file in the crate, so the returned location may point
/// into a different file than the request. Cross-*crate* resolution (external
/// dependencies via `use`) is still unsupported — see `compiler-features.md`.
pub fn goto_definition(current: &Document, krate: &Crate, position: Position) -> Option<Location> {
    let tree = current.tree.as_ref()?;
    let offset = current.line_index.offset(&current.text, position)?;
    let token = analysis::symbol_at(tree, &current.text, offset)?;

    let lookup = krate.lookup();
    let resolved = analysis::resolve(&lookup, &token.symbol)?;

    let path = resolved.source.origin()?;
    let uri = Url::from_file_path(path).ok()?;
    let target_text = resolved.source.content();
    let range = LineIndex::new(target_text).range(target_text, resolved.span);

    Some(Location { uri, range })
}
