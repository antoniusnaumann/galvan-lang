//! `textDocument/codeAction`.
//!
//! Currently offered:
//!
//! - **Add type annotation** (`refactor.rewrite`): writes the inferred type
//!   of an unannotated local binding into the source (`let dog` →
//!   `let dog: Dog`). Offered when the requested range touches the binding's
//!   line. Shares its inference with the inlay hints, so the action inserts
//!   exactly what the hint shows.
//!
//! Diagnostics carry no structured fix information yet, so there are no
//! quickfix actions; new action kinds should follow the pattern here (build
//! a [`WorkspaceEdit`] against the document's URI, gate on the request
//! range).

use std::collections::HashMap;
use std::path::Path;

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Range, TextEdit, Url, WorkspaceEdit,
};

use crate::document::Document;
use crate::features::inlay_hints::unannotated_locals;
use crate::workspace::Crate;

pub fn code_actions(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    range: Range,
) -> Vec<CodeActionOrCommand> {
    let Some(file) = file else {
        return Vec::new();
    };
    let Ok(uri) = Url::from_file_path(file) else {
        return Vec::new();
    };

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
