use derive_more::{Display, From};
use galvan_lexer::LexerString;

#[derive(Debug, Display, PartialEq, Eq, From)]
pub struct Ident(LexerString);

impl Ident {
    pub fn new(name: impl Into<LexerString>) -> Ident {
        Ident(name.into())
    }
}
