//! Implementations of individual LSP features. Each module is a pure function
//! over a [`Document`](crate::document::Document) and request parameters, which
//! keeps the server glue in [`crate::server`] thin and the features unit-testable.

pub mod code_actions;
pub mod completion;
pub mod diagnostics;
pub mod document_highlight;
pub mod folding_range;
pub mod foreign_syntax;
pub mod formatting;
pub mod goto_definition;
pub mod hover;
pub mod inlay_hints;
pub mod references;
pub mod rename;
pub mod selection_range;
pub mod semantic_tokens;
pub mod signature_help;
pub mod symbols;
pub mod type_definition;

use std::collections::HashMap;
use std::path::PathBuf;

use galvan_ast::Span;
use galvan_files::Source;
use tower_lsp::lsp_types::{Location, Url};

use crate::position::LineIndex;

/// Converts byte ranges in crate sources to LSP [`Location`]s, building at
/// most one [`LineIndex`] per source file however many results point into it.
#[derive(Default)]
pub(crate) struct Locations {
    indexes: HashMap<PathBuf, LineIndex>,
}

impl Locations {
    pub fn new() -> Self {
        Self::default()
    }

    /// The location of `start..end` in `source`, or `None` for sources
    /// without an on-disk path (builtins, untitled buffers).
    pub fn location(&mut self, source: &Source, start: usize, end: usize) -> Option<Location> {
        let path = source.origin()?;
        let uri = Url::from_file_path(path).ok()?;
        let text = source.content();
        let index = self
            .indexes
            .entry(path.to_path_buf())
            .or_insert_with(|| LineIndex::new(text));
        Some(Location {
            uri,
            range: index.byte_range(text, start, end),
        })
    }
}

/// Extract a leading `///` doc comment immediately preceding `span`, if any.
///
/// Mirrors the transpiler's own doc-comment handling but is reimplemented here
/// as a tiny text utility (the compiler's version is private). Blank lines and
/// ordinary `//` comments between the doc comment and the declaration are
/// skipped.
pub fn doc_comment(text: &str, span: Span) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if span.start.row == 0 {
        return None;
    }

    let mut doc_lines: Vec<String> = Vec::new();
    let mut row = span.start.row;
    while row > 0 {
        row -= 1;
        let Some(line) = lines.get(row) else { break };
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("///") {
            doc_lines.insert(0, rest.trim().to_string());
        } else if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        } else {
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

/// The source text covered by `span`, trimmed.
pub fn span_text(text: &str, span: Span) -> &str {
    let (start, end) = span.range;
    text.get(start..end).unwrap_or("").trim()
}

/// Whether `byte` can occur in an identifier (`ident` and `type_ident` are
/// both ASCII in the grammar).
pub(crate) fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
