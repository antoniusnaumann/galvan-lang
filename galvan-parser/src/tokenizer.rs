use std::ops::Range;

use galvan_lexer::LexerString;
use galvan_lexer::Token;
use galvan_lexer::TokenExt as GalvanLexerTokenExt;
use galvan_lexer::{Span, SpannedIter};

use crate::TokenError;

pub type Error = TokenError;
pub type Result<T> = std::result::Result<T, Error>;

pub type Tokenizer<'a> = SpannedIter<'a, Token>;

pub trait TokenizerMessage {
    fn current_span(&self) -> Span;

    fn msg(&self, msg: impl Into<String>, annotation: impl Into<String>) -> Error {
        Error {
            msg: msg.into(),
            span: self.current_span().into(),
            annotation: annotation.into(),
        }
    }

    fn invalid_idenfier(&self, msg: impl Into<String>) -> Error {
        self.msg(msg, "Invalid identifier here")
    }

    fn invalid_token(&self) -> Error {
        self.msg("Invalid token", "Invalid token here")
    }

    fn eof(&self, msg: impl Into<String>) -> Error {
        self.msg(msg, "File ends here")
    }

    fn unexpected_token(&self, found: Token, expected: &[&str]) -> Error {
        self.unexpected(format!("Unexpected token: {:?}", found), expected)
    }

    fn unexpected(&self, msg: impl Into<String>, expected: &[&str]) -> Error {
        if expected.is_empty() {
            return self.msg(msg, "Unexpected token");
        }

        let expected = expected
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(", ");
        self.msg(msg, format!("Expected: {expected}")) // TODO: list expected tokens here
    }
}

pub trait TokenizerExt {
    /// Advances the lexer until the next matching token
    /// Supported tokens: (, {, [, ", '
    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<(Token, Span)>>;

    /// Advances the lexer until the given token is encountered, including the token
    fn parse_until_token(&mut self, token: Token) -> Result<Vec<(Token, Span)>>;

    /// Advances the lexer until a non-ignored token is encountered and returns it
    fn parse_ignore_token(&mut self, ignored: Token) -> Result<Option<(Token, Span)>>;
}

pub trait SpannedLexerFromStr {
    fn from_str(s: &str) -> Tokenizer<'_>;
}

impl SpannedLexerFromStr for Tokenizer<'_> {
    fn from_str(s: &str) -> Tokenizer<'_> {
        let lexer = Token::lexer(s);
        lexer.spanned()
    }
}

pub enum MatchingToken {
    SingleQuote,
    DoubleQuote,
    Brace,
    Bracket,
    Paren,
}

impl MatchingToken {
    pub fn opening(&self) -> Token {
        match self {
            MatchingToken::SingleQuote => todo!(),
            MatchingToken::DoubleQuote => todo!(),
            MatchingToken::Brace => Token::BraceOpen,
            MatchingToken::Bracket => Token::BracketOpen,
            MatchingToken::Paren => Token::ParenOpen,
        }
    }

    pub fn closing(&self) -> Token {
        match self {
            MatchingToken::SingleQuote => todo!(),
            MatchingToken::DoubleQuote => todo!(),
            MatchingToken::Brace => Token::BraceClose,
            MatchingToken::Bracket => Token::BracketClose,
            MatchingToken::Paren => Token::ParenClose,
        }
    }
}

impl TokenizerExt for Tokenizer<'_> {
    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<(Token, Span)>> {
        let mut dangling_open = 1;
        let mut tokens = vec![];

        while dangling_open > 0 {
            let (token, span) = self.next().ok_or(self.eof(format!(
                "Expected matching token \"{:?}\" but found end of file!",
                matching.closing()
            )))?;

            let token = token.map_err(|_| self.invalid_token())?;

            if token == matching.closing() {
                dangling_open -= 1;
            } else if token == matching.opening() {
                dangling_open += 1;
            }

            tokens.push((token, span))
        }

        Ok(tokens)
    }

    fn parse_until_token(&mut self, end_token: Token) -> Result<Vec<(Token, Span)>> {
        let mut tokens = vec![];

        loop {
            let (token, span) = self.next().ok_or(self.eof(format!(
                "Expected token {:?} but found end of file!",
                end_token
            )))?;

            let token = token.map_err(|_| self.invalid_token())?;

            if token == end_token {
                tokens.push((token, span));

                return Ok(tokens);
            } else {
                tokens.push((token, span));
            }
        }
    }

    fn parse_ignore_token(&mut self, ignored: Token) -> Result<Option<(Token, Span)>> {
        while let Some((token, span)) = self.next() {
            let token = token.map_err(|_| self.invalid_token())?;
            if token != ignored {
                return Ok(Some((token, span)));
            }
        }

        Ok(None)
    }
}

impl<'a, T> TokenizerExt for TokenIter<'a, T>
where
    T: Iterator<Item = &'a SpannedToken>,
{
    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<(Token, Span)>> {
        let mut dangling_open = 1;
        let mut tokens = vec![];

        while dangling_open > 0 {
            let (token, span) = self.next().ok_or(self.eof(format!(
                "Expected matching token \"{:?}\" but found end of file!",
                matching.closing()
            )))?;

            if *token == matching.closing() {
                dangling_open -= 1;
            } else if *token == matching.opening() {
                dangling_open += 1;
            }

            tokens.push((token.clone(), span.clone()))
        }

        Ok(tokens)
    }

    fn parse_until_token(&mut self, end_token: Token) -> Result<Vec<(Token, Span)>> {
        let mut tokens = vec![];

        loop {
            let (token, span) = self.next().ok_or(self.eof(format!(
                "Expected token {:?} but found end of file!",
                end_token
            )))?;

            if *token == end_token {
                tokens.push((token.clone(), span.clone()));

                return Ok(tokens);
            } else {
                tokens.push((token.clone(), span.clone()));
            }
        }
    }

    fn parse_ignore_token(&mut self, ignored: Token) -> Result<Option<(Token, Span)>> {
        while let Some((token, span)) = self.next() {
            if *token != ignored {
                return Ok(Some((token.clone(), span.clone())));
            }
        }

        Ok(None)
    }
}

impl TokenizerMessage for Tokenizer<'_> {
    fn current_span(&self) -> Span {
        self.span()
    }
}

pub trait SpanInfoForIter {
    fn span_all(&mut self) -> Option<Span>;
    /// Gets the span from the current item to the last item, assuming that all items in between are included there
    fn spanned_error(
        &mut self,
        msg: impl Into<String>,
        annotation: impl Into<String>,
    ) -> TokenError {
        TokenError {
            msg: msg.into(),
            span: self.span_all(),
            annotation: annotation.into(),
        }
    }
}

pub trait SpanInfo {
    fn span_all(&self) -> Option<Span>;
    /// Gets the span from the first item to the last item, assuming that all items in between are included there
    fn spanned_error(&self, msg: impl Into<String>, annotation: impl Into<String>) -> TokenError {
        TokenError {
            msg: msg.into(),
            span: self.span_all(),
            annotation: annotation.into(),
        }
    }
}

impl<T> SpanInfo for T
where
    T: AsRef<[SpannedToken]>,
{
    fn span_all(&self) -> Option<Span> {
        let mut iter = TokenIter::from(self.as_ref().iter());
        iter.span_all()
    }
}

pub type SpannedToken = (Token, Span);
pub struct TokenIter<'a, T>
where
    T: Iterator<Item = &'a SpannedToken>,
{
    iter: T,
    span: Span,
}

impl<'a, T> TokenizerMessage for TokenIter<'a, T>
where
    T: Iterator<Item = &'a SpannedToken>,
{
    fn current_span(&self) -> Span {
        self.span.clone()
    }
}

impl<'a, T> Iterator for TokenIter<'a, T>
where
    T: Iterator<Item = &'a SpannedToken>,
{
    type Item = &'a SpannedToken;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next();
        match next {
            Some((_, ref span)) => {
                self.span = span.clone();
            }
            None => {
                self.span = Span {
                    start: self.span.end,
                    end: self.span.end,
                };
            }
        }

        next
    }
}

impl<'a, T> From<T> for TokenIter<'a, T>
where
    T: Iterator<Item = &'a SpannedToken>,
{
    fn from(iter: T) -> Self {
        TokenIter {
            iter,
            span: Span { start: 0, end: 0 },
        }
    }
}

impl<'a, T> SpanInfoForIter for TokenIter<'a, T>
where
    T: Iterator<Item = &'a SpannedToken>,
{
    fn span_all(&mut self) -> Option<Span> {
        let current = self.next()?;
        let (_, last) = self.last().unwrap_or(current);
        let (_, first) = current;

        Range {
            start: first.start,
            end: last.end,
        }
        .into()
    }
}

pub trait TokenExt {
    /// Ensures that the receiver is a certain token, returns an error otherwise
    fn ensure_token(self, token: Token) -> Result<SpannedToken>;

    /// Ensures that the receiver is a valid identifier token and gets its name, returns an error otherwise
    fn ident(self) -> Result<LexerString>;
}

pub trait OptTokenExt {
    /// Ensures that the receiver is some token, returns an error otherwise
    fn unpack(self) -> Result<SpannedToken>;
}

impl TokenExt for SpannedToken {
    fn ensure_token(self, token: Token) -> Result<SpannedToken> {
        if self.0 != token {
            Err(TokenError {
                msg: format!("Expected token {:#?} but found {:#?}", token, self.0),
                span: self.1.into(),
                annotation: format!("Expected {:#?} here", token),
            })
        } else {
            Ok(self)
        }
    }

    fn ident(self) -> Result<LexerString> {
        match self.0 {
            Token::Ident(name) => Ok(name),
            _ => Err(TokenError {
                msg: "Invalid identifier".to_owned(),
                span: self.1.into(),
                annotation: "Expected identifier here".to_owned(),
            }),
        }
    }
}

impl TokenExt for &SpannedToken {
    fn ensure_token(self, token: Token) -> Result<SpannedToken> {
        self.clone().ensure_token(token)
    }

    fn ident(self) -> Result<LexerString> {
        self.clone().ident()
    }
}

impl TokenExt for Option<SpannedToken> {
    fn ensure_token(self, token: Token) -> Result<SpannedToken> {
        if let Some(t) = self {
            t.ensure_token(token)
        } else {
            Err(TokenError {
                msg: format!("Expected token {:#?} but found none", token),
                // TODO: Return a span that makes sense here (or dont and just return an optional)
                span: None,
                annotation: "".to_owned(),
            })
        }
    }

    fn ident(self) -> Result<LexerString> {
        let t = self.unpack()?;
        t.ident()
    }
}

impl TokenExt for Option<&SpannedToken> {
    fn ensure_token(self, token: Token) -> Result<SpannedToken> {
        self.cloned().ensure_token(token)
    }

    fn ident(self) -> Result<LexerString> {
        self.cloned().ident()
    }
}

impl OptTokenExt for Option<SpannedToken> {
    fn unpack(self) -> Result<SpannedToken> {
        if let Some(t) = self {
            Ok(t)
        } else {
            Err(TokenError {
                msg: "Expected token but found none".to_owned(),
                span: None,
                annotation: "".to_owned(),
            })
        }
    }
}

impl OptTokenExt for Option<&SpannedToken> {
    fn unpack(self) -> Result<SpannedToken> {
        self.cloned().unpack()
    }
}

pub type SpannedParseResult = (std::result::Result<Token, ()>, Span);
impl OptTokenExt for SpannedParseResult {
    fn unpack(self) -> Result<SpannedToken> {
        let (token, span) = self;
        if let Ok(t) = token {
            Ok((t, span))
        } else {
            Err(TokenError {
                msg: "Expected token but found none".to_owned(),
                // TODO: Return a span that makes sense here (or dont and just return an optional)
                span: span.into(),
                annotation: "".to_owned(),
            })
        }
    }
}

pub trait TokensExt {
    fn trim_trailing(&mut self, token: Token);
}

impl TokensExt for Vec<SpannedToken> {
    fn trim_trailing(&mut self, token: Token) {
        while let Some(t) = self.last() {
            if t.0 != token {
                return;
            }

            self.pop();
        }
    }
}
