//! `textDocument/formatting` — full formatting via the `galvan-format` crate.
//!
//! The heavy lifting (token spacing, indentation, line reflow, comment and
//! blank-line preservation) lives in `galvan-format`; this module adapts it
//! to the LSP: the formatted text is diffed line-by-line against the buffer
//! and returned as minimal [`TextEdit`]s, so cursors outside changed regions
//! stay put and range/on-type formatting can filter edits by line.
//!
//! Formatting refuses to run (returns `None`) when the file does not parse:
//! rewriting a broken tree would move code around based on garbage structure.
//!
//! The indent unit follows the request options (`tab_size` spaces, or a
//! tab); everything else uses the canonical Galvan style (see
//! `galvan-format/STYLE.md`).

use galvan_format::{format_source, FormatOptions};
use tower_lsp::lsp_types::{FormattingOptions, Position, Range, TextEdit};

use crate::document::Document;

/// `textDocument/rangeFormatting`: the whole-document edits restricted to
/// the requested lines. The full document is always formatted, so a range
/// format never disagrees with a full format.
pub fn range_formatting(
    current: &Document,
    options: &FormattingOptions,
    range: Range,
) -> Option<Vec<TextEdit>> {
    let edits = formatting(current, options)?;
    Some(
        edits
            .into_iter()
            .filter(|edit| {
                edit.range.start.line >= range.start.line
                    && edit.range.start.line <= range.end.line
            })
            .collect(),
    )
}

/// `textDocument/onTypeFormatting`, triggered by `}`: the edits touching
/// the line the closing bracket was typed on (usually its re-indentation).
pub fn on_type_formatting(
    current: &Document,
    options: &FormattingOptions,
    position: Position,
) -> Option<Vec<TextEdit>> {
    let edits = formatting(current, options)?;
    Some(
        edits
            .into_iter()
            .filter(|edit| edit.range.start.line == position.line)
            .collect(),
    )
}

pub fn formatting(current: &Document, options: &FormattingOptions) -> Option<Vec<TextEdit>> {
    let format_options = FormatOptions {
        indent_width: options.tab_size.max(1) as usize,
        use_tabs: !options.insert_spaces,
        ..FormatOptions::default()
    };
    let formatted = format_source(&current.text, &format_options).ok()?;
    Some(line_edits(current, &formatted))
}

/// Diff the buffer against its formatted form, line by line, and emit
/// per-line edits so range/on-type formatting can filter by line and
/// cursors outside the changed region stay put. Changed lines that pair up
/// are trimmed to the differing span (a re-indented line yields a
/// whitespace-only edit at its start).
fn line_edits(current: &Document, formatted: &str) -> Vec<TextEdit> {
    let original = &current.text;
    let old: Vec<&str> = original.split_inclusive('\n').collect();
    let new: Vec<&str> = formatted.split_inclusive('\n').collect();

    // Byte offset where each original line starts (plus the end of file).
    let mut offsets = Vec::with_capacity(old.len() + 1);
    let mut offset = 0;
    for line in &old {
        offsets.push(offset);
        offset += line.len();
    }
    offsets.push(offset);

    let mut prefix = 0;
    while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old.len() - prefix
        && suffix < new.len() - prefix
        && old[old.len() - 1 - suffix] == new[new.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let old_mid = &old[prefix..old.len() - suffix];
    let new_mid = &new[prefix..new.len() - suffix];

    let mut edits = Vec::new();
    for (old_run, new_run) in diff_runs(old_mid, new_mid) {
        let old_lines: Vec<&str> = old_mid[old_run.clone()].to_vec();
        let new_lines: Vec<&str> = new_mid[new_run].to_vec();
        let first_line = prefix + old_run.start;
        let run_end = prefix + old_run.end;

        let paired = old_lines.len().min(new_lines.len());
        for index in 0..paired {
            if old_lines[index] == new_lines[index] {
                continue;
            }
            let (skip, old_tail, replacement) = trim_line(old_lines[index], new_lines[index]);
            let line_start = offsets[first_line + index];
            edits.push(TextEdit {
                range: current.line_index.byte_range(
                    original,
                    line_start + skip,
                    line_start + old_lines[index].len() - old_tail,
                ),
                new_text: replacement,
            });
        }
        if old_lines.len() > paired {
            // Surplus original lines: delete them.
            edits.push(TextEdit {
                range: current.line_index.byte_range(
                    original,
                    offsets[first_line + paired],
                    offsets[run_end],
                ),
                new_text: String::new(),
            });
        } else if new_lines.len() > paired {
            // Surplus formatted lines: insert them after the paired ones.
            edits.push(TextEdit {
                range: current.line_index.byte_range(
                    original,
                    offsets[run_end],
                    offsets[run_end],
                ),
                new_text: new_lines[paired..].concat(),
            });
        }
    }
    edits
}

/// The byte lengths of the common prefix and suffix of two lines, plus the
/// replacement for the differing middle of the old line.
fn trim_line(old: &str, new: &str) -> (usize, usize, String) {
    let prefix = old
        .char_indices()
        .zip(new.char_indices())
        .find(|((_, old_char), (_, new_char))| old_char != new_char)
        .map(|((index, _), _)| index)
        .unwrap_or_else(|| old.len().min(new.len()));

    let old_rest = &old[prefix..];
    let new_rest = &new[prefix..];
    let suffix = old_rest
        .chars()
        .rev()
        .zip(new_rest.chars().rev())
        .take_while(|(old_char, new_char)| old_char == new_char)
        .map(|(old_char, _)| old_char.len_utf8())
        .sum::<usize>();

    (
        prefix,
        suffix,
        new_rest[..new_rest.len() - suffix].to_string(),
    )
}

/// Aligned runs of differing lines (`old range` ↔ `new range`), from a
/// longest-common-subsequence alignment so edits stay local. Very large
/// diffs fall back to one whole-range run.
fn diff_runs(
    old: &[&str],
    new: &[&str],
) -> Vec<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    if old.is_empty() && new.is_empty() {
        return Vec::new();
    }
    // The document was parseable, so the two sides are usually similar and
    // small. Guard the quadratic table anyway.
    if old.len() * new.len() > 1_000_000 {
        return vec![(0..old.len(), 0..new.len())];
    }

    // lcs[i][j]: length of the LCS of old[i..] and new[j..].
    let mut lcs = vec![vec![0u32; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lcs[i][j] = if old[i] == new[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut runs: Vec<(std::ops::Range<usize>, std::ops::Range<usize>)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    let mut run_start: Option<(usize, usize)> = None;
    loop {
        let matched =
            i < old.len() && j < new.len() && old[i] == new[j] && lcs[i][j] == lcs[i + 1][j + 1] + 1;
        if matched {
            if let Some((old_start, new_start)) = run_start.take() {
                runs.push((old_start..i, new_start..j));
            }
            i += 1;
            j += 1;
            continue;
        }
        let take_old = i < old.len() && (j >= new.len() || lcs[i + 1][j] >= lcs[i][j + 1]);
        let take_new = !take_old && j < new.len();
        if !take_old && !take_new {
            break;
        }
        if run_start.is_none() {
            run_start = Some((i, j));
        }
        if take_old {
            i += 1;
        } else {
            j += 1;
        }
    }
    if let Some((old_start, new_start)) = run_start {
        runs.push((old_start..i, new_start..j));
    }
    runs
}
