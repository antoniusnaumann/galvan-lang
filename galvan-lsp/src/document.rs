//! In-memory representation of an open document and its derived compiler state.

use galvan_ast::SegmentedAsts;
use galvan_files::Source;
use galvan_into_ast::{SegmentAst, SourceIntoAst};
use galvan_parse::{parse_source, ParseTree};

use crate::position::LineIndex;

/// A single open document together with everything we derive from it for
/// answering language requests.
///
/// The compiler stages are run eagerly on every edit; for the file sizes a
/// language server deals with this is cheap and keeps request handling simple
/// and synchronous.
pub struct Document {
    /// The compiler-facing view of the document text.
    pub source: Source,
    /// Tree-sitter parse tree, used to map cursor positions to syntax nodes.
    ///
    /// `None` only if the parser failed catastrophically (it normally always
    /// produces a tree, inserting error nodes for invalid input).
    pub tree: Option<ParseTree>,
    /// The segmented AST (types, functions, ...) used for name resolution.
    ///
    /// `None` if the source could not be lowered into an AST.
    pub segmented: Option<SegmentedAsts>,
    /// Position <-> byte-offset index for the current text.
    pub line_index: LineIndex,
}

impl Document {
    pub fn new(text: impl Into<String>) -> Self {
        let text: String = text.into();
        let line_index = LineIndex::new(&text);
        let source = Source::from_string(text);

        let tree = parse_source(&source).ok();
        let segmented = source
            .clone()
            .try_into_ast()
            .and_then(SegmentAst::segmented)
            .ok();

        Self {
            source,
            tree,
            segmented,
            line_index,
        }
    }

    /// The raw document text.
    pub fn text(&self) -> &str {
        self.source.content()
    }
}
