use pest::iterators::Pairs;
use pest::Parser;
use pest_derive::Parser;
use galvan_parser::Source;

#[derive(Parser)]
#[grammar = "galvan.pest"]
pub struct GalvanParser;

pub type ParsedSource<'a> = Pairs<'a, Rule>;
pub type ParserError = pest::error::Error<Rule>;
pub type ParseResult<'a> = Result<ParsedSource<'a>, ParserError>;
pub fn parse_source(source: &Source) -> ParseResult<'_> {
    let content = source.content();
    let pairs = GalvanParser::parse(Rule::source, content)?;
    Ok(pairs)
}

#[cfg(feature = "exec")]
pub mod exec {
    use std::{env, path::Path};
    use walkdir::WalkDir;

    use galvan_parser::Source;
    use crate::ParseResult;

    pub fn parse_current_dir() -> Vec<(ParseResult<'static>, Source)> {
        let current_dir = env::current_dir().unwrap();
        parse_dir(current_dir)
    }

    pub fn parse_dir(path: impl AsRef<Path>) -> Vec<(ParseResult<'static>, Source)> {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| p.extension() == Some("galvan".as_ref()))
            .map(Source::read)
            .map(Box::new)
            .map(Box::leak)
            .map(|s| (super::parse_source(s), s.clone()))
            .collect::<Vec<_>>()

        // TODO: Aggregate and print errors
    }
}