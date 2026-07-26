//! `textDocument/codeAction`.
//!
//! Currently offered:
//!
//! - **Quickfixes** (`quickfix`): every diagnostic in the request context
//!   that carries a machine-applicable fix (see
//!   [`diagnostics::decode_fix`]) becomes a one-click action — keywords
//!   from other languages (`func` → `fn`), the compiler's did-you-mean
//!   suggestions, and any future fix the compiler attaches.
//! - **Add type annotation** (`refactor.rewrite`): writes the inferred type
//!   of an unannotated local binding into the source (`let dog` →
//!   `let dog: Dog`). Offered when the requested range touches the binding's
//!   line. Shares its inference with the inlay hints, so the action inserts
//!   exactly what the hint shows.

use std::collections::HashMap;
use std::path::Path;

use tower_lsp::lsp_types::{
    CodeAction, CodeActionContext, CodeActionKind, CodeActionOrCommand, Range, TextEdit, Url,
    WorkspaceEdit,
};

use crate::document::Document;
use crate::features::diagnostics;
use crate::features::inlay_hints::unannotated_locals;
use crate::workspace::Crate;

pub fn code_actions(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    range: Range,
    context: &CodeActionContext,
) -> Vec<CodeActionOrCommand> {
    let Some(file) = file else {
        return Vec::new();
    };
    let Ok(uri) = Url::from_file_path(file) else {
        return Vec::new();
    };

    let mut actions = quickfixes(current, &uri, context);
    actions.extend(annotate_locals(current, krate, file, &uri, range));
    actions
}

/// One quickfix per context diagnostic carrying an applicable fix. The
/// client echoes published diagnostics back verbatim, so the fix data
/// round-trips without recomputing the analysis.
fn quickfixes(
    current: &Document,
    uri: &Url,
    context: &CodeActionContext,
) -> Vec<CodeActionOrCommand> {
    context
        .diagnostics
        .iter()
        .filter_map(|diagnostic| {
            let fix = diagnostics::decode_fix(diagnostic)?;
            let found = current.text.get(fix.start..fix.end)?;
            let edit = TextEdit {
                range: current
                    .line_index
                    .byte_range(&current.text, fix.start, fix.end),
                new_text: fix.replacement.clone(),
            };
            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Replace `{found}` with `{}`", fix.replacement),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                is_preferred: Some(true),
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
                    ..Default::default()
                }),
                ..Default::default()
            }))
        })
        .collect()
}

/// "Add type annotation" for unannotated locals whose line the request
/// range touches.
fn annotate_locals(
    current: &Document,
    krate: &Crate,
    file: &Path,
    uri: &Url,
    range: Range,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    for local in unannotated_locals(current, krate, file) {
        let position = current.line_index.position(&current.text, local.insert_at);
        // Offer the action anywhere on the binding's line: the cursor rarely
        // sits exactly on the name when the user reaches for an action.
        if position.line < range.start.line || position.line > range.end.line {
            continue;
        }

        let edit = TextEdit {
            range: Range {
                start: position,
                end: position,
            },
            new_text: local.annotation.clone(),
        };
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Add type annotation `{}` to `{}`", local.annotation, local.name),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
                ..Default::default()
            }),
            ..Default::default()
        }));
    }
    actions
}
