use std::vec::IntoIter;

use galvan_lexer::Token;
use galvan_macro::token;

use crate::*;

use super::parse_type_item;

/// Parses a type definition. This method assumes that modifiers and the type keyword were already consumed
pub fn parse_type(lexer: &mut Tokenizer, mods: &Modifiers) -> Result<TypeDecl> {
    let (token, _span) = lexer
        .next()
        .ok_or(lexer.eof("Expected type name but found end of file."))?;
    let token = token.map_err(|_| lexer.invalid_idenfier("Invalid identifier for type name"))?;

    if let Token::Ident(name) = token {
        let (token, _span) = lexer
            .next()
            .ok_or(lexer.eof("Expected type name but found end of file."))?;
        let token =
            token.map_err(|_| lexer.invalid_idenfier("Invalid identifier for type name"))?;

        // TODO: Parse visibility
        let visibility = Visibility::Inherited;
        let def = match token {
            Token::BraceOpen => {
                let mut tokens = lexer.parse_until_matching(MatchingToken::Brace)?;
                let (_, _) = tokens.pop().ensure_token(Token::BraceClose)?;

                let members = parse_struct_type_members(tokens)?;
                let t = StructTypeDef { members };
                TypeDef::StructType(t)
            }
            Token::ParenOpen => {
                let mut tokens = lexer.parse_until_matching(MatchingToken::Paren)?;
                let (_, _) = tokens.pop().ensure_token(Token::ParenClose)?;

                let members = parse_tuple_type_members(tokens)?;
                let t = TupleTypeDef { members };
                TypeDef::TupleType(t)
            }
            Token::Assign => {
                // TODO: Allow newlines after some symbols like +
                // TODO: Also allow an end of file here
                let tokens = lexer.parse_until_token(Token::Newline)?;

                let aliased_type = parse_type_alias(tokens)?;
                let t = AliasTypeDef {
                    r#type: aliased_type,
                };
                TypeDef::AliasType(t)
            }
            _ => {
                return Err(lexer.unexpected(
                    format!(
                        "Expected one of the following:
                        - type alias:  'type {name} = TypeA'
                        - struct type: 'type {name} {{ attr: TypeA, ... }}'
                        - tuple type:  'type {name}(TypeA, TypeB, ...)
                                
                    ...but found unexpected token instead
                    "
                    ),
                    &[],
                ))
            }
        };

        Ok(TypeDecl {
            visibility,
            def,
            ident: name.into(),
        })
    } else {
        Err(lexer.invalid_idenfier("Invalid identifier for type name at: "))
    }
}

fn parse_struct_type_members(tokens: Vec<SpannedToken>) -> Result<Vec<StructTypeMember>> {
    let mut token_iter: TokenIter<_> = tokens.iter().into();
    let mut members = vec![];
    // TODO: Also allow comma here
    // TODO: Allow directly starting with members without newline
    while let Some(field_name) = token_iter.parse_ignore_token(Token::Newline)? {
        // TODO: parse visibility modifiers and keywords such as ref here, probably parse all until newline to do that
        let field_name = field_name.ident()?;
        let field = Ident::new(field_name);
        let (_, _) = token_iter.next().ensure_token(Token::Colon)?;

        let type_tokens = token_iter.parse_until_token(token!("\n"))?;
        let member_type = parse_type_item(type_tokens)?;

        let member = StructTypeMember {
            visibility: Visibility::Inherited,
            ident: field,
            r#type: member_type,
        };

        members.push(member);
    }

    Ok(members)
}

fn parse_tuple_type_members(tokens: Vec<SpannedToken>) -> Result<Vec<TupleTypeMember>> {
    fn push_member(
        token_iter: &mut IntoIter<SpannedToken>,
        members: &mut Vec<TupleTypeMember>,
    ) -> Result<()> {
        // TODO: parse visibility modifiers and keywords here
        let type_name = token_iter.next().ident()?;
        // TODO: parse all kinds of type items here
        let member_type = TypeItem::plain(type_name);
        let member = TupleTypeMember {
            visibility: Visibility::Inherited,
            r#type: member_type,
        };
        members.push(member);
        Ok(())
    }

    let mut token_iter = tokens.into_iter();
    let mut members = vec![];
    push_member(&mut token_iter, &mut members)?;
    // TODO: Also allow newlines here instead
    // TODO: Allow trailing commas and trailing newlines
    while token_iter.next().ensure_token(Token::Comma).is_ok() {
        push_member(&mut token_iter, &mut members)?;
    }

    Ok(members)
}

fn parse_type_alias(tokens: Vec<SpannedToken>) -> Result<TypeItem> {
    let mut token_iter: TokenIter<_> = tokens.iter().into();
    let type_tokens = token_iter.parse_until_token(token!("\n"))?;
    let member_type = parse_type_item(type_tokens)?;

    Ok(member_type)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_struct_type() -> SourceResult<()> {
        // Note that type keyword is expected to be consumed already
        let src: Source = "TypeA {
    member_b: TypeB
    member_c: TypeC
}"
        .into();
        let mut tokenizer = Tokenizer::from_str(src.content());
        let parsed = parse_type(&mut tokenizer, &Modifiers::default()).with_source(&src)?;

        assert!(matches!(parsed, TypeDecl::StructType(_)));

        Ok(())
    }

    #[test]
    fn test_parse_struct_type_members() -> SourceResult<()> {
        let src: Source = "
a: TypeA
b: TypeB"
            .into();
        let tokenizer = Tokenizer::from_str(src.content());
        let tokens = tokenizer
            .map(|spanned_token| spanned_token.unpack())
            .collect::<Result<Vec<SpannedToken>>>()
            .with_source(&src)?;

        let parsed = parse_struct_type_members(tokens).with_source(&src)?;

        assert!(parsed.len() == 2);
        let a = &parsed[0];
        let b = &parsed[1];

        assert!(matches!(a.visibility, Visibility::Inherited));
        assert!(matches!(b.visibility, Visibility::Inherited));

        assert_eq!(a.ident, Ident::new("a".to_owned()));
        assert_eq!(b.ident, Ident::new("b".to_owned()));

        assert!(matches!(a.r#type, TypeItem::Plain(_)));
        assert!(matches!(b.r#type, TypeItem::Plain(_)));

        match a.r#type {
            TypeItem::Plain(ref inner) => assert_eq!(inner.ident, Ident::new("TypeA".to_owned())),
            _ => unreachable!(),
        }

        match b.r#type {
            TypeItem::Plain(ref inner) => assert_eq!(inner.ident, Ident::new("TypeB".to_owned())),
            _ => unreachable!(),
        }

        Ok(())
    }
}
