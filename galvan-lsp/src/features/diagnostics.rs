//! Syntax diagnostics derived from tree-sitter error and missing nodes.
//!
//! Semantic diagnostics (type errors) are produced by the compiler's
//! typechecker, but it does not yet expose them with source spans in a way the
//! language server can consume — see `compiler-features.md`. Until then we
//! surface syntax errors only.

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity};

use crate::document::Document;

pub fn diagnostics(document: &Document) -> Vec<Diagnostic> {
    let Some(tree) = document.tree.as_ref() else {
        return Vec::new();
    };
    let text = document.text();

    let mut diagnostics = Vec::new();
    let mut cursor = tree.walk();

    // Iterative pre-order traversal over the whole tree.
    let mut recurse = true;
    loop {
        if recurse && cursor.goto_first_child() {
            recurse = true;
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
                range: document.line_index.range(
                    text,
                    galvan_ast::Span {
                        range: (node.start_byte(), node.end_byte().max(node.start_byte() + 1)),
                        start: galvan_ast::Point {
                            row: node.start_position().row,
                            col: node.start_position().column,
                        },
                        end: galvan_ast::Point {
                            row: node.end_position().row,
                            col: node.end_position().column,
                        },
                    },
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
