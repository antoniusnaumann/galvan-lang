//! Diagnostics: syntax errors from tree-sitter plus semantic (type) errors from
//! the compiler's typechecker.
//!
//! Semantic diagnostics come from `galvan_hir::typecheck` run over the whole
//! crate (so cross-file references resolve). Each compiler diagnostic carries
//! the file it belongs to, so we keep only those for the document being
//! refreshed and map their byte spans to ranges in that document.
//!
//! Diagnostics carry machine-readable extras for the rest of the tooling:
//! a stable `code` (the compiler's, or `foreign_keyword` for keywords from
//! other languages) and, when a fix is known, its byte span and replacement
//! in [`Diagnostic::data`]. Code actions read that data back from
//! `context.diagnostics` to offer one-click quickfixes without recomputing
//! the analysis.

use std::path::Path;

use galvan_hir::DiagnosticSeverity as HirSeverity;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString};

use crate::document::Document;
use crate::features::foreign_syntax;
use crate::workspace::Crate;

/// Diagnostic code for a keyword borrowed from another language.
pub const FOREIGN_KEYWORD_CODE: &str = "foreign_keyword";

/// All diagnostics for `document`: parser syntax errors plus, when the document
/// corresponds to a file on disk, the compiler's semantic diagnostics for it.
pub fn diagnostics(document: &Document, krate: &Crate, file: Option<&Path>) -> Vec<Diagnostic> {
    let mut diagnostics = syntax_diagnostics(document);
    if let Some(file) = file {
        diagnostics.extend(semantic_diagnostics(document, krate, file));
        // Foreign keywords that parse as calls (`switch color { … }`,
        // `var x = 1`) never reach the typechecker as errors; report them
        // from the parse tree.
        diagnostics.extend(
            foreign_syntax::foreign_callees(document, krate, file)
                .iter()
                .map(|token| foreign_keyword_diagnostic(document, token)),
        );
    }
    diagnostics
}

/// The diagnostic for one detected foreign keyword, carrying its fix.
fn foreign_keyword_diagnostic(
    document: &Document,
    token: &foreign_syntax::ForeignToken,
) -> Diagnostic {
    Diagnostic {
        range: document
            .line_index
            .byte_range(&document.text, token.start, token.end),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(FOREIGN_KEYWORD_CODE.to_string())),
        source: Some("galvan".to_string()),
        message: foreign_syntax::message(&token.found, token.replacement),
        data: Some(encode_fix(token.start, token.end, token.replacement)),
        ..Default::default()
    }
}

/// A machine-applicable fix decoded from [`Diagnostic::data`]: replace the
/// bytes `start..end` of the document text with `replacement`.
pub struct DiagnosticFix {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

/// Serialize a fix into the value carried by [`Diagnostic::data`].
fn encode_fix(start: usize, end: usize, replacement: &str) -> serde_json::Value {
    serde_json::json!({ "fix": { "start": start, "end": end, "replacement": replacement } })
}

/// The fix carried by `diagnostic`, if any. Inverse of [`encode_fix`].
pub fn decode_fix(diagnostic: &Diagnostic) -> Option<DiagnosticFix> {
    let fix = diagnostic.data.as_ref()?.get("fix")?;
    Some(DiagnosticFix {
        start: fix.get("start")?.as_u64()? as usize,
        end: fix.get("end")?.as_u64()? as usize,
        replacement: fix.get("replacement")?.as_str()?.to_string(),
    })
}

/// Semantic diagnostics for `file`, produced by typechecking the whole crate.
fn semantic_diagnostics(document: &Document, krate: &Crate, file: &Path) -> Vec<Diagnostic> {
    let Some(analysis) = krate.analyze() else {
        return Vec::new();
    };
    analysis
        .diagnostics
        .iter()
        .filter_map(|diagnostic| {
            let span = diagnostic.span.as_ref()?;
            if Path::new(&span.file) != file {
                return None;
            }

            let mut code = diagnostic.code.clone();
            let mut message = match &diagnostic.suggestion {
                Some(suggestion) => format!("{}\n{suggestion}", diagnostic.message),
                None => diagnostic.message.clone(),
            };
            let mut data = diagnostic
                .fix
                .as_ref()
                .filter(|fix| Path::new(&fix.span.file) == file)
                .map(|fix| encode_fix(fix.span.start, fix.span.end, &fix.replacement));

            // An unknown name that is a well-known keyword elsewhere is far
            // more likely a habit from another language than a typo; explain
            // the mapping instead of the generic message (and instead of any
            // Levenshtein guess).
            if matches!(code.as_deref(), Some("unknown_identifier" | "unknown_type")) {
                let name = document.text.get(span.start..span.end).unwrap_or("");
                if let Some(replacement) = foreign_syntax::galvan_equivalent(name) {
                    message = foreign_syntax::message(name, replacement);
                    code = Some(FOREIGN_KEYWORD_CODE.to_string());
                    data = Some(encode_fix(span.start, span.end, replacement));
                }
            }

            Some(Diagnostic {
                range: document
                    .line_index
                    .byte_range(&document.text, span.start, span.end),
                severity: Some(severity(&diagnostic.severity)),
                code: code.map(NumberOrString::String),
                source: Some("galvan".to_string()),
                message,
                data,
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
///
/// When an error region contains keywords from other languages (`func`,
/// `class`, …), the generic "Syntax error" is replaced by one diagnostic per
/// foreign keyword, narrowed to the keyword and carrying its fix — a
/// whole-declaration error range pointing at `func greet() {}` helps nobody.
fn syntax_diagnostics(document: &Document) -> Vec<Diagnostic> {
    let Some(tree) = document.tree.as_ref() else {
        return Vec::new();
    };

    let foreign = foreign_syntax::foreign_tokens(document);
    let mut foreign_used = vec![false; foreign.len()];

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
            let contained: Vec<usize> = (0..foreign.len())
                .filter(|&i| {
                    !foreign_used[i]
                        && foreign[i].start >= node.start_byte()
                        && foreign[i].end <= node.end_byte()
                })
                .collect();

            if contained.is_empty() {
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
            for index in contained {
                foreign_used[index] = true;
                diagnostics.push(foreign_keyword_diagnostic(document, &foreign[index]));
            }
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
