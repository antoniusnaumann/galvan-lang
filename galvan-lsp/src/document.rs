//! In-memory representation of a single open buffer.
//!
//! A `Document` is the *syntactic* view of one file: its text, tree-sitter
//! parse tree (for mapping cursor positions to tokens) and a position index.
//! Semantic resolution spans the whole crate and lives in [`crate::workspace`].

use galvan_files::Source;
use galvan_parse::{parse_source, ParseTree};

use crate::position::LineIndex;

#[derive(Clone)]
pub struct Document {
    /// The raw buffer text.
    pub text: String,
    /// Tree-sitter parse tree.
    ///
    /// `None` only if the parser failed catastrophically (it normally always
    /// produces a tree, inserting error nodes for invalid input).
    pub tree: Option<ParseTree>,
    /// Position <-> byte-offset index for the current text.
    pub line_index: LineIndex,
    /// The client-reported document version this text belongs to (`0` when
    /// unknown). Used to key crate-analysis caching.
    pub version: i32,
}

impl Document {
    pub fn new(text: impl Into<String>) -> Self {
        Self::with_version(text, 0)
    }

    pub fn with_version(text: impl Into<String>, version: i32) -> Self {
        let text: String = text.into();
        let line_index = LineIndex::new(&text);
        let tree = parse_source(&Source::from_string(text.clone())).ok();

        Self {
            text,
            tree,
            line_index,
            version,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}
