use galvan_lexer::{Token, TokenExt};

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

fn lex_input(input: LitStr) -> Token {
    let value = input.value();

    let mut lexer = Token::lexer(value.as_str());
    let token = lexer
        .next()
        .expect("Expected a valid token")
        .expect("Expected a valid token");

    if lexer.next().is_some() {
        panic!("Expected a single token");
    }

    token
}

#[proc_macro]
pub fn token(input: TokenStream) -> TokenStream {
    let token = lex_input(parse_macro_input!(input as LitStr));
    let token_stream: proc_macro2::TokenStream = match token {
        Token::Ident(ident) => {
            panic!("\'{ident}\' is not a valid token. To create an identifier, use `ident!(\"{ident}\")` instead!")
        }
        _ => {
            let debug = format!("galvan_lexer::Token::{:?}", token);
            debug.parse().unwrap()
        }
    };

    token_stream.into()
}

#[proc_macro]
pub fn ident(input: TokenStream) -> TokenStream {
    let token = lex_input(parse_macro_input!(input as LitStr));

    match token {
        Token::Ident(ident) => {
            let lit = LitStr::new(&ident, proc_macro2::Span::call_site());
            quote!(galvan_lexer::Token::Ident(#lit)).into()
        }
        _ => panic!("Cannot create ident with reserved name!"),
    }
}
