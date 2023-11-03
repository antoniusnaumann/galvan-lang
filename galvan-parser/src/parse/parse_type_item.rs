use galvan_macro::token;

use crate::*;

pub fn parse_type_item(tokens: Vec<SpannedToken>) -> Result<TypeItem> {
    let mut tokens = tokens;
    while matches!(tokens.last(), Some((token!("\n"), _))) {
        tokens.pop();
    }
    // TODO: This is not a token error, refactor error types
    parse_type_item_rec(&tokens)
        .ok_or(tokens.spanned_error("Could not parse type", "Type expected here"))
}

fn parse_type_item_rec(tokens: &[SpannedToken]) -> Option<TypeItem> {
    let type_item_parsers = [
        parse_basic_type_item.boxed(),
        parse_array_type_item.boxed(),
        parse_dict_type_item.boxed(),
        parse_ordered_dict_type_item.boxed(),
    ];

    for parser in &type_item_parsers {
        let result = parser.try_parse(tokens);

        if let Some(val) = result {
            return Some(val);
        }
    }

    None
}

fn parse_basic_type_item(tokens: &[SpannedToken]) -> Option<TypeItem> {
    match tokens.get(0) {
        // TODO: ensure that type item is capitalized, otherwise emit a warning
        Some(token) if tokens.len() == 1 => {
            token.ident().map(Ident::new).map(|ident| ident.into()).ok()
        }
        _ => None,
    }
}

fn parse_array_type_item(tokens: &[SpannedToken]) -> Option<TypeItem> {
    parse_enclosed(MatchingToken::Bracket, tokens, |t| {
        parse_type_item_rec(&t).map(|elements| TypeItem::array(elements))
    })
}

fn parse_dict_type_item(tokens: &[SpannedToken]) -> Option<TypeItem> {
    parse_enclosed(MatchingToken::Brace, tokens, |t| {
        let (k, v) = parse_key_value(t)?;
        Some(TypeItem::dict(k, v))
    })
}

fn parse_ordered_dict_type_item(tokens: &[SpannedToken]) -> Option<TypeItem> {
    parse_enclosed(MatchingToken::Bracket, tokens, |t| {
        let (k, v) = parse_key_value(t)?;
        Some(TypeItem::ordered_dict(k, v))
    })
}

fn parse_enclosed(
    delimiters: MatchingToken,
    tokens: &[SpannedToken],
    parse: impl FnOnce(&[SpannedToken]) -> Option<TypeItem>,
) -> Option<TypeItem> {
    match (tokens.get(0), tokens.last()) {
        (Some(first), Some(last))
            if first.ensure_token(delimiters.opening()).is_ok()
                && last.ensure_token(delimiters.closing()).is_ok() =>
        {
            parse(&tokens[1..tokens.len() - 1])
        }
        _ => None,
    }
}

fn parse_key_value(tokens: &[SpannedToken]) -> Option<(TypeItem, TypeItem)> {
    let split: Vec<_> = tokens
        .splitn(2, |(t, _)| *t == token!(":"))
        .take(2)
        .collect();
    let (before_colon, after_colon) = (split.get(0)?, split.get(1)?);
    let key = parse_basic_type_item(before_colon)?;
    let value = parse_type_item_rec(after_colon)?;

    Some((key, value))
}

trait ParseItemType {
    fn try_parse(&self, tokens: &[SpannedToken]) -> Option<TypeItem>;

    fn boxed(self) -> Box<dyn ParseItemType>;
}

impl<F> ParseItemType for F
where
    F: Fn(&[SpannedToken]) -> Option<TypeItem> + 'static,
{
    fn try_parse(&self, tokens: &[SpannedToken]) -> Option<TypeItem> {
        self(tokens)
    }

    fn boxed(self) -> Box<dyn ParseItemType> {
        Box::new(self)
    }
}
