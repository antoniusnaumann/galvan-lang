//! `textDocument/formatting` — whitespace normalization.
//!
//! This is deliberately a *scoped* formatter: it only rewrites whitespace and
//! never reflows tokens across lines, so it cannot change what the program
//! means. The rules:
//!
//! - Each line is indented by one unit per open `{`/`(`/`[` bracket. A line
//!   that begins with closing brackets is dedented to their level, and a
//!   continuation line beginning with `.` / `?.` (a member chain) is indented
//!   one extra unit.
//! - Trailing whitespace is removed (unless the client opts out).
//! - Lines that begin inside a multi-line string literal are untouched, and
//!   bracket counting skips string, char and comment tokens entirely — their
//!   content is data, not structure.
//!
//! Formatting refuses to run (returns `None`) when the file does not parse:
//! reindenting around a syntax error would move code by whatever the broken
//! bracket structure happens to suggest.
//!
//! The indent unit follows the request options (`tab_size` spaces, or a tab).

use tower_lsp::lsp_types::{FormattingOptions, Position, Range, TextEdit};

use crate::document::Document;
use crate::features::is_ident_byte;
use galvan_parse::Node;

/// `textDocument/rangeFormatting`: the whole-document edits restricted to
/// the requested lines. The bracket depth is always computed from the top
/// of the file, so a range format never disagrees with a full format.
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

/// `textDocument/onTypeFormatting`, triggered by `}`: re-indents the line
/// the closing bracket was typed on (the closer usually dedents it). Only
/// the current line is touched; like the document formatter, this does
/// nothing while the file has syntax errors.
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
    let tree = current.tree.as_ref()?;
    if tree.root_node().has_error() {
        return None;
    }

    let text = &current.text;
    let protected = protected_ranges(tree.root_node());
    let unit = if options.insert_spaces {
        " ".repeat(options.tab_size.max(1) as usize)
    } else {
        "\t".to_string()
    };
    let trim_trailing = options.trim_trailing_whitespace.unwrap_or(true);

    let mut edits = Vec::new();
    let mut depth: usize = 0;
    let mut line_start = 0usize;

    while line_start <= text.len() {
        let line_end = text[line_start..]
            .find('\n')
            .map(|i| line_start + i)
            .unwrap_or(text.len());
        let content_end = if text[line_start..line_end].ends_with('\r') {
            line_end - 1
        } else {
            line_end
        };
        let line = &text[line_start..content_end];

        if !inside(&protected, line_start) {
            format_line(
                text,
                line_start,
                content_end,
                line,
                depth,
                &unit,
                trim_trailing,
                &protected,
                current,
                &mut edits,
            );
        }
        depth = update_depth(depth, text, line_start, content_end, &protected);

        if line_end == text.len() {
            break;
        }
        line_start = line_end + 1;
    }

    if let Some(true) = options.insert_final_newline {
        if !text.is_empty() && !text.ends_with('\n') {
            let end = current.line_index.position(text, text.len());
            edits.push(TextEdit {
                range: tower_lsp::lsp_types::Range { start: end, end },
                new_text: "\n".to_string(),
            });
        }
    }

    Some(edits)
}

/// Emit the (at most two) whitespace edits for one line: the leading indent
/// and the trailing whitespace.
#[allow(clippy::too_many_arguments)]
fn format_line(
    text: &str,
    line_start: usize,
    content_end: usize,
    line: &str,
    depth: usize,
    unit: &str,
    trim_trailing: bool,
    protected: &[(usize, usize)],
    current: &Document,
    edits: &mut Vec<TextEdit>,
) {
    let bytes = line.as_bytes();
    let first_content = bytes.iter().position(|b| !matches!(b, b' ' | b'\t'));

    let Some(first_content) = first_content else {
        // Whitespace-only line: reduce to an empty line.
        if trim_trailing && !line.is_empty() {
            edits.push(replace(current, text, line_start, content_end, String::new()));
        }
        return;
    };

    let expected = unit.repeat(indent_level(depth, &line[first_content..]));
    if expected != line[..first_content] {
        edits.push(replace(
            current,
            text,
            line_start,
            line_start + first_content,
            expected,
        ));
    }

    if trim_trailing && !inside(protected, content_end) {
        let trailing = bytes
            .iter()
            .rposition(|b| !matches!(b, b' ' | b'\t'))
            .map(|i| i + 1)
            .unwrap_or(0);
        if trailing < line.len() {
            edits.push(replace(
                current,
                text,
                line_start + trailing,
                content_end,
                String::new(),
            ));
        }
    }
}

/// The indent level of a line given the bracket depth it starts at:
/// leading closing brackets dedent the line to their own level, and a
/// member-chain continuation (`.` / `?.`) indents one extra unit.
fn indent_level(depth: usize, stripped: &str) -> usize {
    let bytes = stripped.as_bytes();

    let mut closers = 0usize;
    for &byte in bytes {
        match byte {
            b')' | b']' | b'}' => closers += 1,
            b' ' | b'\t' => {}
            _ => break,
        }
    }
    // A `.member` / `?.member` chain link. `.` followed by anything else
    // (there are no `.5` literals — the grammar requires a leading digit)
    // is not a continuation.
    let continuation = match bytes {
        [b'.', rest @ ..] | [b'?', b'.', rest @ ..] => {
            rest.first().copied().is_some_and(is_ident_byte)
        }
        _ => false,
    };

    depth.saturating_sub(closers) + usize::from(continuation)
}

/// Bracket depth after the line `[start..end)`, skipping protected tokens.
fn update_depth(depth: usize, text: &str, start: usize, end: usize, protected: &[(usize, usize)]) -> usize {
    let mut depth = depth;
    let bytes = text.as_bytes();
    let mut i = start;
    while i < end {
        if let Some(&(_, range_end)) = protected
            .iter()
            .find(|(range_start, range_end)| *range_start <= i && i < *range_end)
        {
            i = range_end;
            continue;
        }
        match bytes[i] {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    depth
}

/// Byte ranges whose content must not be treated as code structure: string,
/// char and comment tokens. Interpolations inside strings are included —
/// their brackets are balanced, so skipping them cannot skew the depth.
fn protected_ranges(root: Node<'_>) -> Vec<(usize, usize)> {
    fn collect(node: Node<'_>, ranges: &mut Vec<(usize, usize)>) {
        match node.kind() {
            "string_literal" | "raw_string_literal" | "char_literal" | "comment" => {
                ranges.push((node.start_byte(), node.end_byte()));
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    collect(child, ranges);
                }
            }
        }
    }
    let mut ranges = Vec::new();
    collect(root, &mut ranges);
    ranges.sort_unstable();
    ranges
}

/// Whether `offset` falls strictly inside a protected range that began
/// earlier. Used both for line starts (the line continues a multi-line
/// token) and line ends ("trailing whitespace" would be token content).
fn inside(protected: &[(usize, usize)], offset: usize) -> bool {
    protected
        .iter()
        .any(|(start, end)| *start < offset && offset < *end)
}

fn replace(current: &Document, text: &str, start: usize, end: usize, new_text: String) -> TextEdit {
    TextEdit {
        range: current.line_index.byte_range(text, start, end),
        new_text,
    }
}
