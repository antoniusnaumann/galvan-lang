//! `textDocument/references`.

use std::path::Path;

use galvan_files::Source;
use tower_lsp::lsp_types::{Location, Position, Url};

use crate::analysis;
use crate::document::Document;
use crate::position::LineIndex;
use crate::workspace::Crate;

/// Find all references to the symbol at `position`, across every file of the
/// crate.
///
/// References come from the typechecker's symbol index, which records each
/// resolved use while lowering — variable reads, calls, type annotations,
/// constructor calls, field accesses and match patterns alike.
pub fn references(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
    include_declaration: bool,
) -> Vec<Location> {
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

    let mut locations = Vec::new();
    if include_declaration {
        if let Some(site) = analysis::definition_site(definition) {
            locations.extend(location(site.source, site.target.range.0, site.target.range.1));
        }
    }
    for reference in analysis.index.references(id) {
        locations.extend(location(
            &reference.source,
            reference.span.range.0,
            reference.span.range.1,
        ));
    }

    locations
}

fn location(source: &Source, start: usize, end: usize) -> Option<Location> {
    let path = source.origin()?;
    let uri = Url::from_file_path(path).ok()?;
    let text = source.content();
    let range = LineIndex::new(text).byte_range(text, start, end);
    Some(Location { uri, range })
}
