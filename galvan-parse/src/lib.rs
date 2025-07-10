#[cfg(feature = "exec")]
pub mod exec;

use galvan_files::Source;
use thiserror::Error;

pub use tree_sitter::*;

#[derive(Debug, Error)]
#[error("Could not parse source: {path}")]
pub struct ParseError { path: Box<str> }
pub type ParseTree = Tree;
pub type ParseResult = Result<ParseTree, ParseError>;

pub fn parse_source(_source: &Source) -> ParseResult {
    // TODO: Implement tree-sitter parsing once grammar is available
    // For now, return a minimal placeholder
    Err(ParseError { path: "tree-sitter grammar not yet implemented".into() })
}
