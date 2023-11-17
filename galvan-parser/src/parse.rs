use galvan_lexer::Token;
use galvan_macro::token;

use crate::*;

mod parse_type;
pub use parse_type::*;

mod parse_type_item;
pub use parse_type_item::*;

pub type ParsedSource = Vec<RootItem>;

pub fn parse_source(source: &Source) -> Result<ParsedSource> {
    let mut lexer = Tokenizer::from_str(source);
    parse_root(&mut lexer)
}

pub fn parse_root(lexer: &mut Tokenizer<'_>) -> Result<ParsedSource> {
    let mut m = Modifiers::new();
    // TODO: store types, fns, main, etc. separately and add their span
    let mut parsed = Vec::new();

    let allowed = ["fn", "type", "main", "test", "pub", "async", "const"];

    // TODO: Drain the spanned lexer completely into a peekable iterator and store occuring errors
    while let Some(spanned_token) = lexer.next() {
        let (token, _span) = spanned_token;
        let token = token.map_err(|_| lexer.invalid_token())?;

        match token {
            Token::FnKeyword => {
                let parsed_fn = parse_fn(lexer, &m)?;
                m.reset();

                parsed.push(RootItem::Fn(parsed_fn));
            }
            Token::TypeKeyword => {
                let parsed_type = parse_type(lexer, &m)?;
                m.reset();

                parsed.push(RootItem::Type(parsed_type));
            }
            Token::MainKeyword if !m.has_vis_modifier() && !m.has_const_modifier() => {
                let parsed_main = parse_main(lexer, m.asyncness)?;
                m.reset();

                // TODO: Ensure that only one main function is defined
                parsed.push(RootItem::Main(parsed_main));
            }
            Token::TestKeyword if !m.has_vis_modifier() && !m.has_const_modifier() => {
                let parsed_test = parse_test(lexer, m.asyncness)?;
                m.reset();

                parsed.push(RootItem::Test(parsed_test));
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

            Token::Newline => continue,

            // TODO: Add stringified token
            _ => return Err(lexer.unexpected_token(token, &allowed)),
        }
    }

    Ok(parsed)
}

pub fn parse_fn(lexer: &mut Tokenizer, mods: &Modifiers) -> Result<FnDecl> {
    let ident = lexer
        .next()
        .ok_or(lexer.eof("Expected function name but found end of file."))?
        .ident()?;

    // TODO: Handle namespaced functions
    let receiver = None;
    let _open_paren = lexer
        .next()
        .ok_or(lexer.eof("Expected function parameters but found end of file."))?
        .ensure_token(token!("("))?;

    let parameter_list = lexer.parse_until_matching(MatchingToken::Paren)?;
    let parameters = parse_parameter_list(parameter_list)?;

    // TODO: Also handle shorthand functions with -> here
    let return_tokens = lexer.parse_until_token(token!("{"))?;
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

pub fn parse_main(lexer: &mut Tokenizer, asyncness: Async) -> Result<MainDecl> {
    let (token, _span) = lexer
        .next()
        .ok_or(lexer.eof("Expected main body but found end of file."))?;

    let token = token.map_err(|_| lexer.invalid_idenfier("Invalid identifier, expected '{'"))?;

    todo!("Parse main function")
}

pub fn parse_test(lexer: &mut Tokenizer, asyncness: Async) -> Result<TestDecl> {
    let (token, _span) = lexer
        .next()
        .ok_or(lexer.eof("Expected test body or test description but found end of file."))?;

    let token = token.map_err(|_| {
        lexer.invalid_idenfier("Invalid identifier, expected '{' or test description")
    })?;

    todo!("Parse test")
}
