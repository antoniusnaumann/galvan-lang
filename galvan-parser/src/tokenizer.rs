use galvan_lexer::Span;
use galvan_lexer::Token;
use galvan_ast::*;

use crate::TokenError;
use crate::TokenErrorKind;
use crate::TokenIter;

pub type Error = TokenError;
pub type Result<T> = std::result::Result<T, Error>;

pub trait TokenizerExt {
    /// Advances the lexer until the next matching token
    /// Supported tokens: (, {, [, ", '
    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<(Token, Span)>>;

    /// Advances the lexer until the given token is encountered, including the token
    fn parse_until_token(&mut self, token: Token) -> Result<Vec<(Token, Span)>>;

    /// Advances the lexer until a non-ignored token is encountered and returns it
    fn parse_ignore_token(&mut self, ignored: Token) -> Result<Option<(Token, Span)>>;
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

impl<'a> TokenizerExt for TokenIter<'a> {
    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<SpannedToken>> {
        let mut dangling_open = 1;
        let mut tokens = vec![];

        while dangling_open > 0 {
            let (token, span) = self.next().ok_or(TokenError::eof(format!(
                "matching token \"{:?}\"",
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

    fn parse_until_token(&mut self, end_token: Token) -> Result<Vec<SpannedToken>> {
        let mut tokens = vec![];

        loop {
            let (token, span) = self
                .next()
                .ok_or_else(|| TokenError::eof(end_token.clone()))?;

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

pub trait SpanInfo {
    fn span_all(&self) -> Span;
    /// Gets the span from the current item to the last item, assuming that all items in between are included there
    fn spanned_error(
        &self,
        msg: impl Into<String>,
        annotation: impl Into<String>,
        kind: TokenErrorKind,
    ) -> TokenError {
        TokenError {
            msg: Some(msg.into()),
            expected: None,
            kind,
            span: self.span_all(),
        }
    }
}

pub trait IterSpanInfo {
    fn span_all(&mut self) -> Span;
    /// Gets the span from the current item to the last item, assuming that all items in between are included there
    fn spanned_error(
        &mut self,
        msg: impl Into<String>,
        annotation: impl Into<String>,
        kind: TokenErrorKind,
    ) -> TokenError {
        TokenError {
            msg: Some(msg.into()),
            expected: None,
            kind,
            span: self.span_all(),
        }
    }
}

impl SpanInfo for &[SpannedToken] {
    fn span_all(&self) -> Span {
        let mut iter = self.iter();
        iter.span_all()
    }
}

impl IterSpanInfo for TokenIter<'_> {
    fn span_all(&mut self) -> Span {
        let Some(current) = self.next() else {
            return Span { start: 0, end: 0 };
        };
        let (_, last) = self.last().unwrap_or(current);
        let (_, first) = current;

        Span {
            start: first.start,
            end: last.end,
        }
        .into()
    }
}

pub type SpannedToken = (Token, Span);

pub trait TokenExt {
    /// Ensures that the receiver is a certain token, returns an error otherwise
    fn ensure_token(self, expected: Token) -> Result<SpannedToken>;

    /// Ensures that the receiver is a valid identifier token and gets its name, returns an error otherwise
    fn ident(self) -> Result<Ident>;
}

impl TokenExt for SpannedToken {
    fn ensure_token(self, expected: Token) -> Result<SpannedToken> {
        if self.0 != expected {
            Err(TokenError::unexpected(expected, self.1))
        } else {
            Ok(self)
        }
    }

    fn ident(self) -> Result<Ident> {
        match self.0 {
            Token::Ident(name) => Ok(name.into()),
            _ => Err(TokenError::unexpected("identifier", self.1)),
        }
    }
}

impl TokenExt for &SpannedToken {
    fn ensure_token(self, expected: Token) -> Result<SpannedToken> {
        self.clone().ensure_token(expected)
    }

    fn ident(self) -> Result<Ident> {
        self.clone().ident()
    }
}

impl TokenExt for Option<SpannedToken> {
    fn ensure_token(self, expected: Token) -> Result<SpannedToken> {
        match self {
            Some(token) => token.ensure_token(expected),
            None => Err(TokenError::eof(expected)),
        }
    }

    fn ident(self) -> Result<Ident> {
        match self {
            Some(token) => token.ident(),
            None => Err(TokenError::eof("identifier")),
        }
    }
}

impl TokenExt for Option<&SpannedToken> {
    fn ensure_token(self, expected: Token) -> Result<SpannedToken> {
        match self {
            Some(token) => token.ensure_token(expected),
            None => Err(TokenError::eof(expected)),
        }
    }

    fn ident(self) -> Result<Ident> {
        match self {
            Some(token) => token.ident(),
            None => Err(TokenError::eof("identifier")),
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
