//! `textDocument/documentSymbol` and `workspace/symbol`.
//!
//! Both are read straight off the typechecker's symbol index. Document
//! symbols are nested: type declarations carry their fields, enum cases and
//! methods as children; free functions sit at the top level.

use std::path::Path;

use galvan_ast::Span;
use galvan_hir::{Definition, DefinitionKind, SymbolIndex};
use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolInformation, SymbolKind};

use crate::analysis::render_definition;
use crate::document::Document;
use crate::features::Locations;
use crate::workspace::Crate;

/// The nested symbol outline of `file`.
pub fn document_symbols(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
) -> Vec<DocumentSymbol> {
    let Some(file) = file else {
        return Vec::new();
    };
    let Some(analysis) = krate.analyze() else {
        return Vec::new();
    };

    // Definitions of this file, in declaration order (index order).
    let in_file = |definition: &Definition| {
        definition.source.origin() == Some(file) && definition.span != Span::default()
    };

    let mut toplevel: Vec<DocumentSymbol> = Vec::new();
    for (_, definition) in analysis.index.definitions() {
        if !in_file(definition) {
            continue;
        }
        match &definition.kind {
            DefinitionKind::Type => {
                let children = type_members(current, &analysis.index, &definition.name, file);
                let kind = if children
                    .iter()
                    .any(|child| child.kind == SymbolKind::ENUM_MEMBER)
                {
                    SymbolKind::ENUM
                } else {
                    SymbolKind::STRUCT
                };
                toplevel.push(symbol(current, definition, kind, Some(children)));
            }
            DefinitionKind::Function { receiver: None } => {
                toplevel.push(symbol(current, definition, SymbolKind::FUNCTION, None));
            }
            // Members are attached to their owning type; methods on types
            // declared in other files still need a home at the top level.
            DefinitionKind::Function {
                receiver: Some(owner),
            } => {
                let owner_in_file = analysis.index.definitions().any(|(_, def)| {
                    matches!(def.kind, DefinitionKind::Type)
                        && def.name == owner.as_str()
                        && in_file(def)
                });
                if !owner_in_file {
                    toplevel.push(symbol(current, definition, SymbolKind::METHOD, None));
                }
            }
            _ => {}
        }
    }
    toplevel
}

/// Fields, enum cases and methods of the type named `owner` within `file`.
fn type_members(
    current: &Document,
    index: &SymbolIndex,
    owner: &str,
    file: &Path,
) -> Vec<DocumentSymbol> {
    let mut members = Vec::new();
    for (_, definition) in index.definitions() {
        if definition.source.origin() != Some(file) || definition.span == Span::default() {
            continue;
        }
        let kind = match &definition.kind {
            DefinitionKind::Field { owner: o, .. } if o.as_str() == owner => SymbolKind::FIELD,
            DefinitionKind::EnumVariant { owner: o } if o.as_str() == owner => {
                SymbolKind::ENUM_MEMBER
            }
            DefinitionKind::Function {
                receiver: Some(o), ..
            } if o.as_str() == owner => SymbolKind::METHOD,
            _ => continue,
        };
        members.push(symbol(current, definition, kind, None));
    }
    members
}

fn symbol(
    current: &Document,
    definition: &Definition,
    kind: SymbolKind,
    children: Option<Vec<DocumentSymbol>>,
) -> DocumentSymbol {
    // The definition lives in the requested file, which is the current
    // document — use its line index.
    let range = current
        .line_index
        .range(&current.text, definition.decl_span);
    let mut selection_range = current.line_index.range(&current.text, definition.span);
    if !contains_range(range, selection_range) {
        // The protocol requires selection_range ⊆ range.
        selection_range = range;
    }
    DocumentSymbol {
        name: definition.name.clone(),
        detail: Some(render_definition(definition)),
        kind,
        tags: None,
        #[allow(deprecated)]
        deprecated: None,
        range,
        selection_range,
        children,
    }
}

fn contains_range(outer: Range, inner: Range) -> bool {
    position_le(outer.start, inner.start) && position_le(inner.end, outer.end)
}

fn position_le(a: Position, b: Position) -> bool {
    (a.line, a.character) <= (b.line, b.character)
}

/// All crate symbols whose name contains `query` (case-insensitive).
/// An empty query matches everything.
pub fn workspace_symbols(krate: &Crate, query: &str) -> Vec<SymbolInformation> {
    let Some(analysis) = krate.analyze() else {
        return Vec::new();
    };
    let query = query.to_lowercase();
    let mut converter = Locations::new();

    let mut symbols = Vec::new();
    for (_, definition) in analysis.index.definitions() {
        if definition.span == Span::default() {
            continue; // Builtins and synthetic definitions.
        }
        let (kind, container) = match &definition.kind {
            DefinitionKind::Function { receiver: None } => (SymbolKind::FUNCTION, None),
            DefinitionKind::Function {
                receiver: Some(owner),
            } => (SymbolKind::METHOD, Some(owner.as_str())),
            DefinitionKind::Type => (SymbolKind::STRUCT, None),
            DefinitionKind::Field { owner, .. } => (SymbolKind::FIELD, Some(owner.as_str())),
            DefinitionKind::EnumVariant { owner } => {
                (SymbolKind::ENUM_MEMBER, Some(owner.as_str()))
            }
            // Locals and parameters are not workspace symbols.
            DefinitionKind::Local { .. } | DefinitionKind::Parameter { .. } => continue,
        };
        if !query.is_empty() && !definition.name.to_lowercase().contains(&query) {
            continue;
        }
        let Some(location) = converter.location(
            &definition.source,
            definition.span.range.0,
            definition.span.range.1,
        ) else {
            continue;
        };
        #[allow(deprecated)]
        symbols.push(SymbolInformation {
            name: definition.name.clone(),
            kind,
            tags: None,
            deprecated: None,
            location,
            container_name: container.map(str::to_owned),
        });
    }
    symbols
}
