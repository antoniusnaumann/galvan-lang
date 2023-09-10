use arc_lexer::Token;
use logos::{Lexer, Span};

pub type Error = (String, Span);
pub type Result<T> = std::result::Result<T, Error>;

pub fn parse_source(lexer: &mut Lexer<'_, Token>) -> Result<()> {
    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::FnKeyword) => {
                todo!("Parse function declaration")
            }
            Ok(Token::TypeKeyword) => {
                todo!("Parse type declaration")
            }
            Ok(Token::MainKeyword) => {
                todo!("Parse main function")
            }
            Ok(Token::TestKeyword) => {
                todo!("Parse test block")
            }
            Ok(Token::BuildKeyword) => {
                return Err((
                    "The build keyword is reserved but currently not implemented yet. Found at: "
                        .to_owned(),
                    lexer.span(),
                ));
            }
            // TODO: Add stringified token
            _ => return Err(("Unexpected token at: ".to_owned(), lexer.span())),
            // TODO: handle the case that a visibility modifier appears
            // TODO: handle visibility modifier
        }
    }

    Ok(())
}
