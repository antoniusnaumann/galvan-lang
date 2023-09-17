use arc_lexer::Token;
use logos::{Lexer, Span};

pub type Error = (String, Span);
pub type Result<T> = std::result::Result<T, Error>;

pub type Tokenizer<'a> = Lexer<'a, Token>;

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
    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<Token>>;

    /// Advances the lexer until the given token is encountered
    fn parse_until_token(&mut self, token: Token) -> Result<Vec<Token>>;
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

    fn parse_until_matching(&mut self, matching: MatchingToken) -> Result<Vec<Token>> {
        let mut dangling_open = 1;
        let mut tokens = vec![];

        while dangling_open > 0 {
            let token = self
                .next()
                .ok_or(self.msg("Expected matching token but found end of file!"))?
                .map_err(|_| self.unexpected_token())?;

            if token == matching.closing() {
                dangling_open -= 1;
            } else if token == matching.opening() {
                dangling_open += 1;
            }

            tokens.push(token)
        }

        Ok(tokens)
    }

    fn parse_until_token(&mut self, token: Token) -> Result<Vec<Token>> {
        let mut tokens = vec![];

        Ok(tokens)
    }
}
