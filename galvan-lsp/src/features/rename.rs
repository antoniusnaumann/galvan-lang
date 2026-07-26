//! `textDocument/rename` and `textDocument/prepareRename`.
//!
//! Rename is mechanical: the definition's identifier token plus every
//! recorded reference is replaced with the new name. Only symbols with a
//! real source location can be renamed (builtins cannot).

use std::collections::HashMap;
use std::path::Path;

use galvan_ast::Span;
use tower_lsp::lsp_types::{Position, PrepareRenameResponse, TextEdit, Url, WorkspaceEdit};

use crate::analysis;
use crate::document::Document;
use crate::features::Locations;
use crate::workspace::Crate;

/// The range (and current text) of the symbol at `position`, if it can be
/// renamed.
pub fn prepare_rename(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
) -> Option<PrepareRenameResponse> {
    let offset = current.line_index.offset(&current.text, position)?;
    let (analysis, file) = (krate.analyze()?, file?);
    let (id, definition) = analysis::symbol_at(&analysis.index, file, offset)?;
    if definition.span == Span::default() {
        return None; // Builtins and synthetic bindings have no location.
    }

    let span = token_span_at(&analysis.index, id, file, offset)?;
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range: current.line_index.range(&current.text, span),
        placeholder: definition.name.clone(),
    })
}

/// A workspace edit renaming the symbol at `position` to `new_name`, across
/// every file of the crate.
pub fn rename(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    if !is_valid_identifier(new_name) {
        return None;
    }
    let offset = current.line_index.offset(&current.text, position)?;
    let (analysis, file) = (krate.analyze()?, file?);
    let (id, definition) = analysis::symbol_at(&analysis.index, file, offset)?;
    if definition.span == Span::default() {
        return None;
    }

    let mut converter = Locations::new();
    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    let mut add = |location: Option<tower_lsp::lsp_types::Location>| {
        if let Some(location) = location {
            let edits = changes.entry(location.uri).or_default();
            if !edits.iter().any(|edit| edit.range == location.range) {
                edits.push(TextEdit {
                    range: location.range,
                    new_text: new_name.to_string(),
                });
            }
        }
    };

    add(converter.location(
        &definition.source,
        definition.span.range.0,
        definition.span.range.1,
    ));
    for reference in analysis.index.references(id) {
        add(converter.location(
            &reference.source,
            reference.span.range.0,
            reference.span.range.1,
        ));
    }

    if changes.is_empty() {
        return None;
    }
    Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

/// The span of the identifier token under the cursor (the definition's own
/// token or the reference the cursor is on), used as the rename range.
fn token_span_at(
    index: &galvan_hir::SymbolIndex,
    id: galvan_hir::DefinitionId,
    file: &Path,
    offset: usize,
) -> Option<Span> {
    let definition = index.definition(id);
    if definition.source.origin() == Some(file) && covers(definition.span, offset) {
        return Some(definition.span);
    }
    index
        .references(id)
        .filter(|reference| reference.source.origin() == Some(file))
        .find(|reference| covers(reference.span, offset))
        .map(|reference| reference.span)
}

/// End-inclusive containment, matching the index's point queries.
fn covers(span: Span, offset: usize) -> bool {
    span.range.0 <= offset && offset <= span.range.1
}

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
