use arc_lexer::Token;
use logos::{Logos, Span, SpannedIter};

pub type Error = (String, Span);
pub type Result<T> = std::result::Result<T, Error>;

pub type Tokenizer<'a> = SpannedIter<'a, Token>;

pub trait TokenizerExt {
    fn err<S, T>(&self, msg: S) -> Result<T>
    where
        S: Into<String>;
    fn msg<S>(&self, msg: S) -> Error
    where
        S: Into<String>;
    fn unexpected_token(&self) -> Error;

    /// Advances the lexer until the next matching token
    /// Supported tokens: (, {, [, ", '
    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<(Token, Span)>>;

    /// Advances the lexer until the given token is encountered
    fn parse_until_token(&mut self, token: Token) -> Result<Vec<(Token, Span)>>;

    fn from_str<'a>(s: &'a str) -> Tokenizer<'a>;
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
    fn err<S, T>(&self, msg: S) -> Result<T>
    where
        S: Into<String>,
    {
        Err(self.msg(msg))
    }

    fn msg<S>(&self, msg: S) -> Error
    where
        S: Into<String>,
    {
        (msg.into(), self.span())
    }

    fn unexpected_token(&self) -> Error {
        self.msg("Unexpected token at:")
    }

    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<(Token, Span)>> {
        let mut dangling_open = 1;
        let mut tokens = vec![];

        while dangling_open > 0 {
            let (token, span) = self
                .next()
                .ok_or(self.msg("Expected matching token but found end of file!"))?;

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

    fn parse_until_token(&mut self, token: Token) -> Result<Vec<(Token, Span)>> {
        let mut tokens = vec![];
        todo!("Implement");
        Ok(tokens)
    }

    fn from_str<'a>(s: &'a str) -> Tokenizer<'a> {
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
            Err((
                format!("Expected token {:#?} but found {:#?} at:", token, self.0),
                self.1,
            ))
        } else {
            Ok(self)
        }
    }

    fn ident(self) -> Result<String> {
        match self.0 {
            Token::Ident(name) => Ok(name),
            _ => Err((format!("Invalid identifier at:"), self.1)),
        }
    }
}

impl TokenExt for Option<SpannedToken> {
    fn ensure_token(self, token: Token) -> Result<SpannedToken> {
        if let Some(t) = self {
            t.ensure_token(token)
        } else {
            // TODO: Return a span that makes sense here (or dont and just return an optional)
            Err((format!("Expected token {:#?} but found none", token), 0..0))
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
            // TODO: Return a span that makes sense here (or dont and just return an optional)
            Err(("Expected token but found none".to_owned(), 0..0))
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
            Err(("Expected token but found none".to_owned(), span))
        }
    }
}
