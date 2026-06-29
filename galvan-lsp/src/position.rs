//! Conversions between LSP positions (line / UTF-16 character) and byte offsets
//! into the document text.
//!
//! Galvan spans ([`galvan_ast::Span`]) and tree-sitter nodes both work in byte
//! offsets, so byte offsets are the common currency: LSP requests are converted
//! to a byte offset before querying the syntax tree, and compiler spans are
//! converted back to LSP ranges for responses.

use galvan_ast::Span;
use tower_lsp::lsp_types::{Position, Range};

/// Maps between byte offsets and `(line, utf-16 character)` positions for a
/// single document revision.
pub struct LineIndex {
    /// Byte offset at which each line starts. Always begins with `0`.
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (idx, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self { line_starts }
    }

    /// Convert an LSP position into a byte offset into `text`.
    ///
    /// Returns `None` if the line is out of range. Characters past the end of a
    /// line are clamped to the line end.
    pub fn offset(&self, text: &str, pos: Position) -> Option<usize> {
        let line_start = *self.line_starts.get(pos.line as usize)?;
        let line_end = self
            .line_starts
            .get(pos.line as usize + 1)
            .copied()
            .unwrap_or(text.len());

        let line = &text[line_start..line_end];
        let mut utf16 = 0u32;
        for (byte_idx, ch) in line.char_indices() {
            if utf16 >= pos.character {
                return Some(line_start + byte_idx);
            }
            utf16 += ch.len_utf16() as u32;
        }
        Some(line_end)
    }

    /// Convert a byte offset into an LSP position.
    pub fn position(&self, text: &str, offset: usize) -> Position {
        let offset = offset.min(text.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next - 1,
        };
        let line_start = self.line_starts[line];

        let mut utf16 = 0u32;
        for (byte_idx, ch) in text[line_start..].char_indices() {
            if line_start + byte_idx >= offset {
                break;
            }
            utf16 += ch.len_utf16() as u32;
        }
        Position {
            line: line as u32,
            character: utf16,
        }
    }

    /// Convert a byte range into an LSP range.
    pub fn byte_range(&self, text: &str, start: usize, end: usize) -> Range {
        Range {
            start: self.position(text, start),
            end: self.position(text, end),
        }
    }

    /// Convert a Galvan byte-range span into an LSP range.
    pub fn range(&self, text: &str, span: Span) -> Range {
        self.byte_range(text, span.range.0, span.range.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_ascii() {
        let text = "fn main {\n    foo()\n}\n";
        let index = LineIndex::new(text);

        let pos = Position {
            line: 1,
            character: 4,
        };
        let offset = index.offset(text, pos).unwrap();
        assert_eq!(&text[offset..offset + 3], "foo");
        assert_eq!(index.position(text, offset), pos);
    }

    #[test]
    fn handles_multibyte() {
        // "café" -> 'é' is two UTF-8 bytes but one UTF-16 unit.
        let text = "let x = \"café\"\nfoo";
        let index = LineIndex::new(text);
        let start_of_foo = text.find("foo").unwrap();
        let pos = index.position(text, start_of_foo);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
        assert_eq!(index.offset(text, pos).unwrap(), start_of_foo);
    }
}
