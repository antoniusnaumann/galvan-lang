use crate::*;

pub fn parse_type_item(tokens: Vec<SpannedToken>) -> Result<TypeItem> {
    // TODO: This is not a token error, refactor error types
    parse_type_item_rec(&tokens)
        .ok_or(tokens.spanned_error("Could not parse type", "Type expected here"))
}

fn parse_type_item_rec(tokens: &[SpannedToken]) -> Option<TypeItem> {
    let type_item_parsers = [parse_basic_type_item.boxed()];

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
        Some(token) if tokens.len() == 1 => token
            .ident()
            .map(|s| Ident::new(s))
            .map(|ident| BasicTypeItem { ident }.into())
            .ok(),
        _ => None,
    }
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
