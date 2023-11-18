use galvan_lexer::{LexerError, Span};

use crate::{AnnotationType, AsParserMessage, ParserMessage, Source, SourceAnnotation};

pub enum TokenErrorKind {
    InvalidToken,
    UnexpectedToken,
    EndOfFile,
}

impl TokenErrorKind {
    pub fn message(&self) -> &'static str {
        match self {
            TokenErrorKind::InvalidToken => "Invalid token",
            TokenErrorKind::UnexpectedToken => "Unexpected token",
            TokenErrorKind::EndOfFile => "Expected token but found end of file!",
        }
    }

    pub fn default_annotation(&self) -> &'static str {
        match self {
            TokenErrorKind::InvalidToken => "Invalid token here",
            TokenErrorKind::UnexpectedToken => "Unexpected token here",
            TokenErrorKind::EndOfFile => "File ends here",
        }
    }
}

pub struct TokenError {
    pub msg: Option<String>,
    pub expected: Option<String>,
    pub span: Span,
    pub kind: TokenErrorKind,
}

impl TokenError {
    pub fn message(&self) -> String {
        match self.msg {
            Some(ref message) => format!("{}: {}", self.kind.message(), message),
            None => self.kind.message().to_owned(),
        }
    }

    pub fn annotation(&self) -> String {
        match self.expected {
            Some(ref expected) => format!("Expected {expected} here"),
            None => self.kind.default_annotation().into(),
        }
    }

    pub fn with_expected(self, expected: impl ToString) -> Self {
        TokenError {
            expected: Some(expected.to_string()),
            ..self
        }
    }

    pub fn eof(expected: impl ToString) -> TokenError {
        TokenError {
            msg: None,
            expected: Some(expected.to_string()),
            // TODO: Meaningful Span
            span: Span {
                start: usize::MAX,
                end: usize::MAX,
            },
            kind: TokenErrorKind::EndOfFile,
        }
    }

    pub fn unexpected(expected: impl ToString, span: Span) -> TokenError {
        TokenError {
            msg: None,
            expected: Some(expected.to_string()),
            span,
            kind: TokenErrorKind::UnexpectedToken,
        }
    }
}

impl From<LexerError> for TokenError {
    fn from(value: LexerError) -> Self {
        TokenError {
            msg: None,
            span: value.span,
            kind: TokenErrorKind::InvalidToken,
            expected: None,
        }
    }
}

impl AsParserMessage for TokenError {
    fn as_parser_message(&self, src: Source) -> ParserMessage<'_> {
        ParserMessage {
            issue: self.message().into(),
            hint: None,
            msg_type: AnnotationType::Error,
            src,
            // TODO: Create annotations
            annotations: vec![SourceAnnotation {
                label: self.kind.default_annotation(),
                range: (self.span.start, self.span.end),
                annotation_type: AnnotationType::Error,
            }],
        }
    }
}
