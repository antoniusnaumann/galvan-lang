//! `textDocument/inlayHint`: inferred types of local bindings.
//!
//! A hint like `: Dog` is shown after the name of every local binding that
//! has no explicit type annotation in the source (`let`/`mut`/`ref`
//! declarations, loop variables, match and else-block bindings). The types
//! come from the typechecker's symbol index.

use std::path::Path;

use galvan_ast::{Span, TypeElement};
use galvan_hir::DefinitionKind;
use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position, Range};

use crate::document::Document;
use crate::workspace::Crate;

pub fn inlay_hints(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    range: Range,
) -> Vec<InlayHint> {
    let Some(file) = file else {
        return Vec::new();
    };
    let Some(analysis) = krate.analyze() else {
        return Vec::new();
    };

    let mut hints = Vec::new();
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

        let position = current
            .line_index
            .position(&current.text, definition.span.range.1);
        if !position_in(position, range) {
            continue;
        }
        hints.push(InlayHint {
            position,
            label: InlayHintLabel::String(format!(": {ty}")),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
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
