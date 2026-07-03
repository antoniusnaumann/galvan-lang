//! `textDocument/definition`.

use std::path::Path;

use galvan_files::Source;
use tower_lsp::lsp_types::{Location, Position};

use crate::analysis;
use crate::document::Document;
use crate::features::Locations;
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
        if let Some((_, definition)) = analysis::symbol_at(&analysis.index, file, offset) {
            let site = analysis::definition_site(definition)?;
            return location(site.source, site.target.range.0, site.target.range.1);
        }
        // A file with a parse error is silently absent from the analysis;
        // only give up here when the file actually participated in it.
        if krate.file_parses(file) {
            return None;
        }
    }

    // Fallback: resolve the token under the cursor by name.
    let tree = current.tree.as_ref()?;
    let token = analysis::token_at(tree, &current.text, offset)?;
    let lookup = krate.lookup();
    let resolved = analysis::resolve(&lookup, &token.token)?;
    location(resolved.source, resolved.span.range.0, resolved.span.range.1)
}

fn location(source: &Source, start: usize, end: usize) -> Option<Location> {
    Locations::new().location(source, start, end)
}
