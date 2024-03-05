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

pub fn parse_source(source: &Source) -> ParseResult {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_galvan::language())
        .expect("Error loading Galvan grammar!");

    let content = source.content();
    parser.parse(content, None).ok_or_else(|| { 
        let file = match source {
            Source::File { path, content: _, canonical_name: _ } => path.to_string_lossy(),
            Source::Str(_) => "{input string}".into(),
            Source::Missing => "{missing}".into(),
            Source::Builtin => "{builtin}".into(),

        };
        ParseError { path: file.into() } })
}
