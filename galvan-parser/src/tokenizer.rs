use galvan_lexer::Token;
use logos::{Logos, Span, SpannedIter};

use crate::TokenError;

pub type Error = TokenError;
pub type Result<T> = std::result::Result<T, Error>;

pub type Tokenizer<'a> = SpannedIter<'a, Token>;

pub trait TokenizerExt {
    fn msg(&self, msg: impl Into<String>, annotation: impl Into<String>) -> Error;

    fn invalid_idenfier(&self, msg: impl Into<String>) -> Error {
        self.msg(msg, "Invalid identifier here")
    }

    fn eof(&self, msg: impl Into<String>) -> Error {
        self.msg(msg, "File ends here")
    }

    fn unexpected_token(&self) -> Error {
        self.unexpected("Unexpected token")
    }

    fn unexpected(&self, msg: impl Into<String>) -> Error {
        self.msg(msg, "Expected ") // TODO: list expected tokens here
    }

    /// Advances the lexer until the next matching token
    /// Supported tokens: (, {, [, ", '
    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<(Token, Span)>>;

    /// Advances the lexer until the given token is encountered, including the token
    fn parse_until_token(&mut self, token: Token) -> Result<Vec<(Token, Span)>>;

    fn from_str(s: &str) -> Tokenizer<'_>;
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
    fn msg(&self, msg: impl Into<String>, annotation: impl Into<String>) -> Error {
        Error {
            msg: msg.into(),
            span: self.span(),
            annotation: annotation.into(),
        }
    }

    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<(Token, Span)>> {
        let mut dangling_open = 1;
        let mut tokens = vec![];

        while dangling_open > 0 {
            let (token, span) = self
                .next()
                .ok_or(self.eof("Expected matching token but found end of file!"))?;

            let token = token.map_err(|_| self.unexpected_token())?;

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

            let token = token.map_err(|_| self.unexpected_token())?;

            if token == end_token {
                tokens.push((token, span));

                return Ok(tokens);
            } else {
                tokens.push((token, span));
            }
        }
    }

    fn from_str(s: &str) -> Tokenizer<'_> {
        let lexer = Token::lexer(s);
        lexer.spanned()
    }
}

pub type SpannedToken = (Token, Span);
pub trait TokenExt {
    /// Ensures that the receiver is a certain token, returns an error otherwise
    fn ensure_token(self, token: Token) -> Result<SpannedToken>;

    /// Ensures that the receiver is a valid identifier token and gets its name, returns an error otherwise
    fn ident(self) -> Result<String>;
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

    fn ident(self) -> Result<String> {
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

    fn ident(self) -> Result<String> {
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
