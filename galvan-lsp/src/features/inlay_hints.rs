//! `textDocument/inlayHint`: inferred types of local bindings.
//!
//! A hint like `: Dog` is shown after the name of every local binding that
//! has no explicit type annotation in the source (`let`/`mut`/`ref`
//! declarations, loop variables, match and else-block bindings). The types
//! come from the typechecker's symbol index. Each hint carries the text edit
//! that inserts the annotation, so clients can apply it directly; the same
//! edit is offered as a code action (see [`super::code_actions`]).

use std::path::Path;

use galvan_ast::{Span, TypeElement};
use galvan_hir::DefinitionKind;
use tower_lsp::lsp_types::{
    InlayHint, InlayHintKind, InlayHintLabel, Position, Range, TextEdit,
};

use crate::document::Document;
use crate::workspace::Crate;

/// A local binding without a type annotation in the source, together with the
/// annotation the typechecker inferred for it.
pub(crate) struct UnannotatedLocal {
    pub name: String,
    /// Byte offset directly after the binding's name, where `: Type` goes.
    pub insert_at: usize,
    /// The rendered annotation text, e.g. `: Dog`.
    pub annotation: String,
}

/// All local bindings in `file` that have no explicit type annotation but
/// whose type the typechecker inferred (and could name).
pub(crate) fn unannotated_locals(
    current: &Document,
    krate: &Crate,
    file: &Path,
) -> Vec<UnannotatedLocal> {
    let Some(analysis) = krate.analyze() else {
        return Vec::new();
    };

    let mut locals = Vec::new();
    for (_, definition) in analysis.index.definitions() {
        let DefinitionKind::Local { ty, .. } = &definition.kind else {
            continue;
        };
        if definition.source.origin() != Some(file) || definition.span == Span::default() {
            continue;
        }
        // Types the checker could not (or need not) name are no help.
        if matches!(ty, TypeElement::Infer(_) | TypeElement::Void(_)) {
            continue;
        }
        if has_explicit_annotation(&current.text, definition.span) {
            continue;
        }
        locals.push(UnannotatedLocal {
            name: definition.name.clone(),
            insert_at: definition.span.range.1,
            annotation: format!(": {ty}"),
        });
    }
    locals
}

pub fn inlay_hints(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    range: Range,
) -> Vec<InlayHint> {
    let Some(file) = file else {
        return Vec::new();
    };

    let mut hints = Vec::new();
    for local in unannotated_locals(current, krate, file) {
        let position = current.line_index.position(&current.text, local.insert_at);
        if !position_in(position, range) {
            continue;
        }
        hints.push(InlayHint {
            position,
            label: InlayHintLabel::String(local.annotation.clone()),
            kind: Some(InlayHintKind::TYPE),
            text_edits: Some(vec![TextEdit {
                range: Range {
                    start: position,
                    end: position,
                },
                new_text: local.annotation,
            }]),
            tooltip: None,
            padding_left: None,
            padding_right: None,
            data: None,
        });
    }
    hints
}

/// Whether the binding's name is directly followed by a `:` type annotation
/// in the source, in which case a hint would just repeat it.
fn has_explicit_annotation(text: &str, ident: Span) -> bool {
    text[ident.range.1.min(text.len())..]
        .chars()
        .find(|ch| !ch.is_whitespace())
        == Some(':')
}

fn position_in(position: Position, range: Range) -> bool {
    let point = (position.line, position.character);
    (range.start.line, range.start.character) <= point
        && point <= (range.end.line, range.end.character)
}
