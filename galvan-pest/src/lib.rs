extern crate core;


#[cfg(feature = "exec")]
pub mod exec;
mod source;
pub use source::*;

use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::*;

#[derive(Parser)]
#[grammar = "galvan.pest"]
pub struct GalvanParser;

pub struct Span(usize, usize);
pub type BorrowedSpan<'a> = pest::Span<'a>;

impl From<pest::Span<'_>> for Span {
    fn from(value: pest::Span) -> Self {
        Span(value.start(), value.end())
    }
}

pub type ParserNodes<'a> = Pairs<'a, Rule>;
pub type ParserError = Box<pest::error::Error<Rule>>;
pub type ParseResult<'a> = Result<ParserNodes<'a>, ParserError>;

pub type ParserNode<'a> = Pair<'a, Rule>;


pub fn parse_source(source: &Source) -> ParseResult<'_> {
    let content = source.content();
    let pairs = GalvanParser::parse(Rule::source, content)?;
    Ok(pairs)
}

