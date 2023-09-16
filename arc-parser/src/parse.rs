use arc_lexer::Token;
use logos::{Lexer, Span};

use crate::{Async, Const, Modifiers, Visibility as Vis};

pub type Error = (String, Span);
pub type Result<T> = std::result::Result<T, Error>;

pub fn parse_source(lexer: &mut Lexer<'_, Token>) -> Result<()> {
    let mut m = Modifiers::new();

    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::FnKeyword) => {
                parse_fn(lexer, &m);
                m.reset();
            }
            Ok(Token::TypeKeyword) => {
                parse_type(lexer, &m);
                m.reset();
            }
            Ok(Token::MainKeyword) if !m.has_vis_modifier() && !m.has_const_modifier() => {
                parse_main(lexer, m.asyncness);
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
            Ok(Token::PublicKeyword) if !m.has_vis_modifier() => m.visibility = Vis::Public,
            Ok(Token::ConstKeyword) if !m.has_const_modifier() => m.constness = Const::Const,
            Ok(Token::AsyncKeyword) if !m.has_async_modifier() => m.asyncness = Async::Async,

            // TODO: Add stringified token
            _ => return Err(("Unexpected token at: ".to_owned(), lexer.span())),
        }
    }

    Ok(())
}

pub fn parse_type(lexer: &mut Lexer<'_, Token>, mods: &Modifiers) {
    todo!("Parse type declaration")
}

pub fn parse_fn(lexer: &mut Lexer<'_, Token>, mods: &Modifiers) {
    todo!("Parse function declaration")
}

pub fn parse_main(lexer: &mut Lexer<'_, Token>, asyncness: Async) {
    todo!("Parse main function")
}
