pub mod token;
pub use token::*;

pub use logos::Logos as TokenExt;
pub use logos::{Lexer, Span, SpannedIter};
pub type LexerString = std::sync::Arc<str>;
