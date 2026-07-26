//! `textDocument/selectionRange` — "expand selection".
//!
//! The chain of syntax-tree ancestors of the position, deduplicated by
//! range, so each expansion step selects the next-larger enclosing
//! construct (identifier → expression → statement → body → declaration →
//! file).

use tower_lsp::lsp_types::{Position, SelectionRange};

use crate::document::Document;

pub fn selection_range(current: &Document, position: Position) -> Option<SelectionRange> {
    let offset = current.line_index.offset(&current.text, position)?;
    let tree = current.tree.as_ref()?;
    let mut node = tree
        .root_node()
        .descendant_for_byte_range(offset, offset)?;

    let mut chain = vec![(node.start_byte(), node.end_byte())];
    while let Some(parent) = node.parent() {
        chain.push((parent.start_byte(), parent.end_byte()));
        node = parent;
    }

    // Build outermost-first so each range's `parent` is the next-larger one.
    let mut result: Option<SelectionRange> = None;
    for (start, end) in chain.into_iter().rev() {
        let range = current.line_index.byte_range(&current.text, start, end);
        if result.as_ref().is_some_and(|outer| outer.range == range) {
            continue;
        }
        result = Some(SelectionRange {
            range,
            parent: result.map(Box::new),
        });
    }
    result
}
