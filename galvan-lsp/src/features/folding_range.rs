//! `textDocument/foldingRange`.
//!
//! Purely syntactic, from the tree-sitter parse tree:
//!
//! - any node whose brace/bracket/paren pair spans multiple lines folds as a
//!   region, from the opener's line to the line *before* the closer (so the
//!   closing token stays visible when collapsed);
//! - runs of `//` comments on consecutive lines fold as comments;
//! - runs of `use` declarations on consecutive lines fold as imports.

use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};

use crate::document::Document;

pub fn folding_ranges(current: &Document) -> Vec<FoldingRange> {
    let Some(tree) = current.tree.as_ref() else {
        return Vec::new();
    };

    let mut ranges = Vec::new();
    let mut comment_rows = Vec::new();
    let mut import_rows = Vec::new();

    let mut cursor = tree.walk();
    let mut recurse = true;
    loop {
        if recurse && cursor.goto_first_child() {
            continue;
        }
        let node = cursor.node();

        match node.kind() {
            "comment" => comment_rows.push(node.start_position().row),
            "use_declaration" => import_rows.push(node.start_position().row),
            _ => {}
        }
        if let Some(range) = bracket_fold(&node) {
            ranges.push(range);
        }

        if cursor.goto_next_sibling() {
            recurse = true;
        } else if cursor.goto_parent() {
            recurse = false;
        } else {
            break;
        }
    }

    ranges.extend(row_runs(comment_rows, FoldingRangeKind::Comment));
    ranges.extend(row_runs(import_rows, FoldingRangeKind::Imports));
    ranges.sort_by_key(|range| (range.start_line, range.end_line));
    ranges
}

/// Fold for a node whose bracket pair spans multiple lines: from the
/// opener's line to the line before the closer.
fn bracket_fold(node: &galvan_parse::Node) -> Option<FoldingRange> {
    let mut cursor = node.walk();
    let opener = node
        .children(&mut cursor)
        .find(|child| matches!(child.kind(), "brace_open" | "bracket_open" | "paren_open"))?;
    let start_line = opener.start_position().row;
    let end_line = node.end_position().row.checked_sub(1)?;
    (end_line > start_line).then_some(FoldingRange {
        start_line: start_line as u32,
        end_line: end_line as u32,
        kind: Some(FoldingRangeKind::Region),
        ..Default::default()
    })
}

/// One fold per run of two or more consecutive rows.
fn row_runs(mut rows: Vec<usize>, kind: FoldingRangeKind) -> Vec<FoldingRange> {
    rows.sort_unstable();
    rows.dedup();

    let mut ranges = Vec::new();
    let mut run: Option<(usize, usize)> = None;
    for row in rows {
        run = match run {
            Some((start, end)) if row == end + 1 => Some((start, row)),
            Some((start, end)) => {
                if end > start {
                    ranges.push(fold(start, end, kind.clone()));
                }
                Some((row, row))
            }
            None => Some((row, row)),
        };
    }
    if let Some((start, end)) = run {
        if end > start {
            ranges.push(fold(start, end, kind));
        }
    }
    ranges
}

fn fold(start: usize, end: usize, kind: FoldingRangeKind) -> FoldingRange {
    FoldingRange {
        start_line: start as u32,
        end_line: end as u32,
        kind: Some(kind),
        ..Default::default()
    }
}
