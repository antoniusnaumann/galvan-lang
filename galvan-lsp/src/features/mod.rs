//! Implementations of individual LSP features. Each module is a pure function
//! over a [`Document`](crate::document::Document) and request parameters, which
//! keeps the server glue in [`crate::server`] thin and the features unit-testable.

pub mod completion;
pub mod diagnostics;
pub mod goto_definition;
pub mod hover;

use galvan_ast::Span;

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
