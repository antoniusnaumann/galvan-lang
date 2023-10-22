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
            span: self.current_span(),
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
            let (token, span) = self
                .next()
                .ok_or(self.eof("Expected matching token but found end of file!"))?;

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
            let (token, span) = self
                .next()
                .ok_or(self.eof("Expected matching token but found end of file!"))?;

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

impl TokenizerMessage for Tokenizer<'_> {
    fn current_span(&self) -> Span {
        self.span()
    }
}

pub type SpannedToken = (Token, Span);
pub struct TokenIter<T>
where
    T: Iterator<Item = SpannedToken>,
{
    iter: T,
    span: Span,
}

impl<T> TokenizerMessage for TokenIter<T>
where
    T: Iterator<Item = SpannedToken>,
{
    fn current_span(&self) -> Span {
        self.span.clone()
    }
}

impl<T> Iterator for TokenIter<T>
where
    T: Iterator<Item = SpannedToken>,
{
    type Item = SpannedToken;

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

impl<T> From<T> for TokenIter<T>
where
    T: Iterator<Item = SpannedToken>,
{
    fn from(iter: T) -> Self {
        TokenIter {
            iter,
            span: Span { start: 0, end: 0 },
        }
    }
}

impl<T> TokenizerExt for TokenIter<T>
where
    T: Iterator<Item = SpannedToken>,
{
    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<(Token, Span)>> {
        todo!()
    }

    fn parse_until_token(&mut self, token: Token) -> Result<Vec<(Token, Span)>> {
        todo!()
    }

    fn parse_ignore_token(&mut self, ignored: Token) -> Result<Option<(Token, Span)>> {
        Ok(self.find(|(t, _)| *t != ignored))
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
                span: self.1,
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
                span: self.1,
                annotation: "Invalid identifier here".to_owned(),
            }),
        }
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
                span: 0..0,
                annotation: "".to_owned(),
            })
        }
    }

    fn ident(self) -> Result<LexerString> {
        let t = self.unpack()?;
        t.ident()
    }
}

impl OptTokenExt for Option<SpannedToken> {
    fn unpack(self) -> Result<SpannedToken> {
        if let Some(t) = self {
            Ok(t)
        } else {
            Err(TokenError {
                msg: "Expected token but found none".to_owned(),
                // TODO: Return a span that makes sense here (or dont and just return an optional)
                span: 0..0,
                annotation: "".to_owned(),
            })
        }
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
                span,
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
