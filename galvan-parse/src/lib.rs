#[cfg(feature = "exec")]
pub mod exec;

use galvan_files::Source;
use tree_sitter::{Parser, Tree};

pub type ParseResult = Result<ParseTree, ParseError>;
pub type ParseError = ();
pub type ParseTree = Tree;

pub fn parse_source(source: &Source) -> ParseResult {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_galvan::language()).expect("Error loading Galvan grammar!");

    let content = source.content();
    parser.parse(content, None).ok_or(())
}
