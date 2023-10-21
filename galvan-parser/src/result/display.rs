use annotate_snippets::display_list::DisplayList;

use crate::{AsParserMessage, ParserMessage, Source, TokenError};

pub trait FormattedOutput {
    fn as_formatted_output(&self) -> DisplayList;
}

impl FormattedOutput for ParserMessage<'_> {
    fn as_formatted_output(&self) -> DisplayList {
        let snippet = self.as_snippet();

        DisplayList::from(snippet)
    }
}

pub type SourceResult<T> = Result<T, SourceError>;
pub type SourceError = WithSource<TokenError>;

pub trait ToSourceResult<T> {
    fn with_source(self, src: &Source) -> SourceResult<T>;
}

impl<T> ToSourceResult<T> for Result<T, TokenError> {
    fn with_source(self, src: &Source) -> SourceResult<T> {
        self.map_err(|err| err.with_source(src.clone()))
    }
}

pub struct WithSource<T> {
    pub value: T,
    pub source: Source,
}

pub trait ItemWithSource: Sized {
    fn with_source(self, src: Source) -> WithSource<Self>;
}

impl<T> ItemWithSource for T
where
    T: AsParserMessage,
{
    fn with_source(self, src: Source) -> WithSource<Self> {
        WithSource {
            value: self,
            source: src,
        }
    }
}

impl<T> From<(Source, T)> for WithSource<T>
where
    T: AsParserMessage,
{
    fn from((source, value): (Source, T)) -> WithSource<T> {
        WithSource { value, source }
    }
}

impl<T> From<(T, Source)> for WithSource<T>
where
    T: AsParserMessage,
{
    fn from((value, source): (T, Source)) -> WithSource<T> {
        WithSource { value, source }
    }
}

impl<T> std::fmt::Debug for WithSource<T>
where
    T: AsParserMessage,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "\n\n{}\n",
            self.value
                .as_parser_message(self.source.clone())
                .as_formatted_output()
        )
    }
}

impl<T> std::fmt::Display for WithSource<T>
where
    T: AsParserMessage,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            self.value
                .as_parser_message(self.source.clone())
                .as_formatted_output()
        )
    }
}
