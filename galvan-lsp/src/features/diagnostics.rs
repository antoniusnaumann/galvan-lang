//! Diagnostics: syntax errors from tree-sitter plus semantic (type) errors from
//! the compiler's typechecker.
//!
//! Semantic diagnostics come from `galvan_hir::typecheck` run over the whole
//! crate (so cross-file references resolve). Each compiler diagnostic carries
//! the file it belongs to, so we keep only those for the document being
//! refreshed and map their byte spans to ranges in that document.

use std::path::Path;

use galvan_hir::DiagnosticSeverity as HirSeverity;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity};

use crate::document::Document;
use crate::workspace::Crate;

/// All diagnostics for `document`: parser syntax errors plus, when the document
/// corresponds to a file on disk, the compiler's semantic diagnostics for it.
pub fn diagnostics(document: &Document, krate: &Crate, file: Option<&Path>) -> Vec<Diagnostic> {
    let mut diagnostics = syntax_diagnostics(document);
    if let Some(file) = file {
        diagnostics.extend(semantic_diagnostics(document, krate, file));
    }
    diagnostics
}

/// Semantic diagnostics for `file`, produced by typechecking the whole crate.
fn semantic_diagnostics(document: &Document, krate: &Crate, file: &Path) -> Vec<Diagnostic> {
    let Some(analysis) = krate.analyze() else {
        return Vec::new();
    };
    analysis
        .diagnostics
        .into_iter()
        .filter_map(|diagnostic| {
            let span = diagnostic.span?;
            if Path::new(&span.file) != file {
                return None;
            }

            let message = match &diagnostic.suggestion {
                Some(suggestion) => format!("{}\n{suggestion}", diagnostic.message),
                None => diagnostic.message.clone(),
            };

            Some(Diagnostic {
                range: document
                    .line_index
                    .byte_range(&document.text, span.start, span.end),
                severity: Some(severity(&diagnostic.severity)),
                source: Some("galvan".to_string()),
                message,
                ..Default::default()
            })
        })
        .collect()
}

fn severity(severity: &HirSeverity) -> DiagnosticSeverity {
    match severity {
        HirSeverity::Error => DiagnosticSeverity::ERROR,
        HirSeverity::Warning => DiagnosticSeverity::WARNING,
        HirSeverity::Info => DiagnosticSeverity::INFORMATION,
    }
}

/// Syntax diagnostics derived from tree-sitter error and missing nodes.
fn syntax_diagnostics(document: &Document) -> Vec<Diagnostic> {
    let Some(tree) = document.tree.as_ref() else {
        return Vec::new();
    };

    let mut diagnostics = Vec::new();
    let mut cursor = tree.walk();

    // Iterative pre-order traversal over the whole tree.
    let mut recurse = true;
    loop {
        if recurse && cursor.goto_first_child() {
            continue;
        }

        let node = cursor.node();
        if node.is_error() || node.is_missing() {
            let message = if node.is_missing() {
                format!("Missing {}", node.kind())
            } else {
                "Syntax error".to_string()
            };
            diagnostics.push(Diagnostic {
                range: document.line_index.byte_range(
                    &document.text,
                    node.start_byte(),
                    node.end_byte().max(node.start_byte() + 1),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("galvan".to_string()),
                message,
                ..Default::default()
            });
        }

        if cursor.goto_next_sibling() {
            recurse = true;
        } else if cursor.goto_parent() {
            recurse = false;
        } else {
            break;
        }
    }

    diagnostics
}
