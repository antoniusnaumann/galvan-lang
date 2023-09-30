use arc_lexer::Token;

use crate::*;

mod parse_type;
pub use parse_type::*;

// TODO: Introduce type for parsed source
type ParsedSource = ();

pub fn parse_source(lexer: &mut Tokenizer<'_>) -> Result<ParsedSource> {
    let mut m = Modifiers::new();

    // TODO: Drain the spanned lexer completely into a peekable iterator and store occuring errors
    while let Some(spanned_token) = lexer.next() {
        let (token, _span) = spanned_token;
        let token = token.map_err(|_| lexer.unexpected_token())?;

        match token {
            Token::FnKeyword => {
                parse_fn(lexer, &m)?;
                m.reset();
            }
            Token::TypeKeyword => {
                parse_type(lexer, &m)?;
                m.reset();
            }
            Token::MainKeyword if !m.has_vis_modifier() && !m.has_const_modifier() => {
                parse_main(lexer, m.asyncness)?;
                m.reset();
            }
            Token::TestKeyword if !m.has_vis_modifier() && !m.has_const_modifier() => {
                parse_test(lexer, m.asyncness)?;
                m.reset();
            }
            Token::BuildKeyword => {
                return Err(lexer.msg(
                    "The build keyword is reserved but currently not implemented yet.",
                    "keyword used here",
                ));
            }

            Token::PublicKeyword if !m.has_vis_modifier() => m.visibility = Visibility::Public,
            Token::ConstKeyword if !m.has_const_modifier() => m.constness = Const::Const,
            Token::AsyncKeyword if !m.has_async_modifier() => m.asyncness = Async::Async,

            // TODO: Add stringified token
            _ => return Err(lexer.unexpected_token()),
        }
    }

    Ok(())
}

pub fn parse_fn(lexer: &mut Tokenizer, mods: &Modifiers) -> Result<FnDecl> {
    let (token, _span) = lexer
        .next()
        .ok_or(lexer.eof("Expected function name but found end of file."))?;

    let token = token.map_err(|_| lexer.invalid_idenfier("Invalid identifier for type name"))?;

    todo!("Parse function declaration")
}

pub fn parse_main(lexer: &mut Tokenizer, asyncness: Async) -> Result<()> {
    let (token, _span) = lexer
        .next()
        .ok_or(lexer.eof("Expected main body but found end of file."))?;

    let token = token.map_err(|_| lexer.invalid_idenfier("Invalid identifier, expected '{'"))?;

    todo!("Parse main function")
}

pub fn parse_test(lexer: &mut Tokenizer, asyncness: Async) -> Result<()> {
    let (token, _span) = lexer
        .next()
        .ok_or(lexer.eof("Expected test body or test description but found end of file."))?;

    let token = token.map_err(|_| {
        lexer.invalid_idenfier("Invalid identifier, expected '{' or test description")
    })?;

    todo!("Parse main function")
}
