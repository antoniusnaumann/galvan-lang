use std::vec::IntoIter;

use arc_lexer::Token;

use crate::*;

/// Parses a type definition. This method assumes that modifiers and the type keyword were already consumed
pub fn parse_type(lexer: &mut Tokenizer, mods: &Modifiers) -> Result<TypeDecl> {
    let token = lexer
        .next()
        .ok_or(lexer.msg("Expected type name but found end of file."))?
        .map_err(|_| lexer.msg("Invalid identifier for type name at: "))?;

    if let Token::Ident(name) = token {
        let token = lexer
            .next()
            .ok_or(lexer.msg("Expected type name but found end of file."))?
            .map_err(|_| lexer.msg("Invalid identifier for type name at: "))?;

        match token {
            Token::BraceOpen => {
                let mut tokens = lexer.parse_until_matching(crate::MatchingToken::Brace)?;
                let (brace_close, span) = tokens.pop().ensure_token(Token::BraceClose)?;

                let members = parse_struct_type_members(tokens);
                let t = StructTypeDecl { members };
                Ok(TypeDecl::StructType(t))
            }
            Token::ParenOpen => {
                let mut tokens = lexer.parse_until_matching(crate::MatchingToken::Paren)?;
                let paren_close = tokens.pop();
                assert_eq!(paren_close, Some(Token::ParenClose));

                let members = parse_tuple_type_members(tokens);
                let t = TupleTypeDecl { members };
                Ok(TypeDecl::TupleType(t))
            }
            Token::Assign => {
                // TODO: Allow newlines after some symbols like +
                // TODO: Also allow an end of file here
                let mut tokens = lexer.parse_until_token(Token::Newline)?;
                let new_line = tokens.pop();
                assert_eq!(new_line, Some(Token::Newline));

                let aliased_type = parse_type_alias(tokens);
                let t = AliasTypeDecl {
                    r#type: aliased_type,
                };
                Ok(TypeDecl::AliasType(t))
            }
            _ => lexer.err(format!(
                "Expected one of the following:
                        - type alias:  'type {name} = TypeA'
                        - struct type: 'type {name} {{ attr: TypeA, ... }}'
                        - tuple type:  'type {name}(TypeA, TypeB, ...)
                                
                    ...but found unexpected token instead at:
                    "
            )),
        }
    } else {
        lexer.err("Invalid identifier for type name at: ")
    }
}

fn parse_struct_type_members(tokens: Vec<SpannedToken>) -> Result<Vec<StructTypeMember>> {
    let mut token_iter = tokens.into_iter();
    let mut members = vec![];
    while let Some(name) = token_iter.next() {
        // TODO: parse visibility modifiers and keywords such as ref here, probably parse all until newline to do that

        let field_name = Ident::new(name.ident()?);
        let (_, _) = token_iter.next().ensure_token(Token::Colon)?;
        let type_name = token_iter.next().ident()?;

        let member_type = TypeItem::Plain(BasicTypeItem {
            ident: Ident::new(type_name),
        });

        let member = StructTypeMember {
            visibility: Visibility::Inherited,
            ident: field_name,
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
    push_member(&mut token_iter, &mut members);

    while let Some(name) = token_iter.next() {
        // TODO: also allow newlines here
        let _ = token_iter.next().ensure_token(Token::Comma);
        push_member(&mut token_iter, &mut members);
    }

    Ok(members)
}

fn parse_type_alias(tokens: Vec<SpannedToken>) -> TypeItem<BasicTypeItem> {
    todo!()
}
