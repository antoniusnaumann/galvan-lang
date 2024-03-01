use galvan_files::Source;

pub type ParseResult = Result<ParseTree, ParseError>;
pub type ParseError = ();
pub struct ParseTree;

pub fn parse_source(source: &Source) -> ParseResult {
    todo!()
}
