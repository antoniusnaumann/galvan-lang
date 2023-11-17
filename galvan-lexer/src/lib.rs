pub mod token;
pub use token::*;

pub use logos::Logos as TokenExt;
pub use logos::{Lexer, Span, SpannedIter};
pub type LexerString = std::sync::Arc<str>;

pub struct LexerError {
    pub span: Span,
    error: token::Error,
}

pub type SpannedToken = (Token, Span);
// TODO: Error recovery
pub fn lex(source: &str) -> Result<Vec<SpannedToken>, LexerError> {
    let lexer = Lexer::new(source);
    lexer
        .spanned()
        .map(|(t, span)| match t {
            Ok(token) => Ok((token, span)),
            Err(error) => Err(LexerError { span, error }),
        })
        .collect()
}
