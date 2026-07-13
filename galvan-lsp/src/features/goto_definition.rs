//! `textDocument/definition`.

use std::path::{Path, PathBuf};

use galvan_files::Source;
use galvan_hir::RustLocation;
use tower_lsp::lsp_types::{Location, Position, Range, Url};

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
            if let Some(rust) = analysis::rust_location(definition) {
                return rust_source_location(rust, analysis.manifest_dir.as_deref());
            }
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

/// A location inside the Rust sources of an imported crate. rustdoc records
/// 1-based lines and 0-based columns; LSP positions are 0-based on both.
/// Relative paths (path dependencies) resolve against the consumer project.
fn rust_source_location(rust: &RustLocation, manifest_dir: Option<&Path>) -> Option<Location> {
    let path: PathBuf = if rust.path.is_absolute() {
        rust.path.clone()
    } else {
        manifest_dir?.join(&rust.path)
    };
    if !path.is_file() {
        return None;
    }
    let position = Position {
        line: rust.line.saturating_sub(1) as u32,
        character: rust.column as u32,
    };
    Some(Location {
        uri: Url::from_file_path(&path).ok()?,
        range: Range {
            start: position,
            end: position,
        },
    })
}
