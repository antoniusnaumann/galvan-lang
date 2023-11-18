use std::slice::Iter;

use galvan_lexer::{lex, Token};
use galvan_macro::token;

mod parse_type;
pub use parse_type::*;

mod parse_type_item;
pub use parse_type_item::*;

use crate::*;

pub type ParsedSource = Vec<RootItem>;
pub type TokenIter<'a> = Iter<'a, SpannedToken>;

pub fn parse_source(source: &Source) -> Result<ParsedSource> {
    let lexed = lex(source.content())?;
    parse_root(&lexed)
}

pub fn parse_root(tokens: &[SpannedToken]) -> Result<ParsedSource> {
    let mut m = Modifiers::new();
    // TODO: store types, fns, main, etc. separately and add their span
    let mut parsed = Vec::new();

    let allowed = ["fn", "type", "main", "test", "pub", "async", "const"];

    let mut token_iter = tokens.iter();
    // TODO: Peek here instead and use peekable iterator to not consume first token
    while let Some(spanned_token) = token_iter.next() {
        let (token, span) = spanned_token;

        match token {
            Token::FnKeyword => {
                let parsed_fn = parse_fn(&mut token_iter, &m)?;
                m.reset();

                parsed.push(RootItem::Fn(parsed_fn));
            }
            Token::TypeKeyword => {
                let parsed_type = parse_type(&mut token_iter, &m)?;
                m.reset();

                parsed.push(RootItem::Type(parsed_type));
            }
            Token::MainKeyword if !m.has_vis_modifier() && !m.has_const_modifier() => {
                let parsed_main = parse_main(&mut token_iter, m.asyncness)?;
                m.reset();

                // TODO: Ensure that only one main function is defined
                parsed.push(RootItem::Main(parsed_main));
            }
            Token::TestKeyword if !m.has_vis_modifier() && !m.has_const_modifier() => {
                let parsed_test = parse_test(&mut token_iter, m.asyncness)?;
                m.reset();

                parsed.push(RootItem::Test(parsed_test));
            }
            Token::BuildKeyword => {
                return Err(TokenError {
                    msg: Some(
                        "The build keyword is reserved but currently not implemented yet.".into(),
                    ),
                    expected: None,
                    span: span.clone(),
                    kind: TokenErrorKind::InvalidToken,
                });
            }

            Token::PublicKeyword if !m.has_vis_modifier() => m.visibility = Visibility::Public,
            Token::ConstKeyword if !m.has_const_modifier() => m.constness = Const::Const,
            Token::AsyncKeyword if !m.has_async_modifier() => m.asyncness = Async::Async,

            Token::Newline => continue,

            // TODO: Add stringified token
            _ => {
                return Err(TokenError::unexpected(
                    format!("one of: {}", &allowed.join(", ")),
                    span.clone(),
                ))
            }
        }
    }

    Ok(parsed)
}

pub fn parse_fn(token_iter: &mut TokenIter<'_>, mods: &Modifiers) -> Result<FnDecl> {
    let ident = token_iter
        .next()
        .ok_or_else(|| TokenError::eof("function name"))?
        .ident()?;

    // TODO: Handle namespaced functions
    let receiver = None;
    let _open_paren = token_iter
        .next()
        .ok_or(TokenError::eof("function parameters"))?
        .ensure_token(token!("("))?;

    let parameter_list = token_iter.parse_until_matching(MatchingToken::Paren)?;
    let parameters = parse_parameter_list(parameter_list)?;

    // TODO: Also handle shorthand functions with -> here
    let return_tokens = token_iter.parse_until_token(token!("{"))?;
    let return_type = parse_return_type(return_tokens)?;

    let signature = FnSignature::new(mods.clone(), receiver, ident, parameters, return_type);
    // TODO: Parse block. Probably also let a block have a return statement and nested blocks
    let block = Block { statements: vec![] };

    Ok(FnDecl { signature, block })
}

fn parse_parameter_list(parameter_list: Vec<SpannedToken>) -> Result<ParamList> {
    todo!()
}

fn parse_return_type(return_tokens: Vec<SpannedToken>) -> Result<Option<ReturnType>> {
    todo!()
}

pub fn parse_main(token_iter: &mut TokenIter<'_>, asyncness: Async) -> Result<MainDecl> {
    let (token, _span) = token_iter.next().ok_or(TokenError::eof("main body"))?;

    todo!("Parse main function")
}

pub fn parse_test(token_iter: &mut TokenIter<'_>, asyncness: Async) -> Result<TestDecl> {
    let (token, _span) = token_iter
        .next()
        .ok_or_else(|| TokenError::eof("test body or test description"))?;

    todo!("Parse test")
}
