//! `textDocument/definition`.

use std::path::Path;

use galvan_files::Source;
use tower_lsp::lsp_types::{Location, Position, Url};

use crate::analysis;
use crate::document::Document;
use crate::position::LineIndex;
use crate::workspace::Crate;

/// Resolve the declaration of the symbol at `position`.
///
/// Resolution goes through the typechecker's symbol index, so locals,
/// parameters, methods, fields, enum variants and types all resolve — across
/// every file of the crate. When the crate does not typecheck, top-level
/// functions and types still resolve by name.
pub fn goto_definition(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
) -> Option<Location> {
    let offset = current.line_index.offset(&current.text, position)?;

    if let (Some(analysis), Some(file)) = (krate.analyze(), file) {
        let (_, definition) = analysis::symbol_at(&analysis.index, file, offset)?;
        let site = analysis::definition_site(definition)?;
        return location(site.source, site.target.range.0, site.target.range.1);
    }

    // Fallback: resolve the token under the cursor by name.
    let tree = current.tree.as_ref()?;
    let token = analysis::token_at(tree, &current.text, offset)?;
    let lookup = krate.lookup();
    let resolved = analysis::resolve(&lookup, &token.token)?;
    location(resolved.source, resolved.span.range.0, resolved.span.range.1)
}

fn location(source: &Source, start: usize, end: usize) -> Option<Location> {
    let path = source.origin()?;
    let uri = Url::from_file_path(path).ok()?;
    let text = source.content();
    let range = LineIndex::new(text).byte_range(text, start, end);
    Some(Location { uri, range })
}
