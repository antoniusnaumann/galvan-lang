use crate::{Ident, SingleExpression};
use from_pest::pest::iterators::Pairs;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;

#[derive(Debug, PartialEq, Eq)]
pub struct MemberChain {
    pub elements: Vec<SingleExpression>,
}

impl MemberChain {
    pub fn is_field(&self) -> bool {
        self.elements
            .last()
            .is_some_and(|e| matches!(e, SingleExpression::Ident(_)))
    }

    pub fn field(&self) -> Option<&SingleExpression> {
        self.elements.last().and_then(|e| match e {
            SingleExpression::Ident(_) => Some(e),
            _ => None,
        })
    }

    pub fn field_ident(&self) -> Option<&Ident> {
        self.elements.last().and_then(|e| match e {
            SingleExpression::Ident(i) => Some(i),
            _ => None,
        })
    }
}

impl FromPest<'_> for MemberChain {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(ConversionError::NoMatch)?;
        if pair.as_rule() != Rule::member_chain {
            return Err(ConversionError::NoMatch);
        }

        // println!("Member chain base: {:#?}", pair);
        let mut pairs = pair.into_inner();
        let mut elements = Vec::new();
        while let Some(_) = pairs.peek() {
            // println!("Member chain element: {:#?}", pair);
            // println!("Member chain: {:#?}", elements);
            elements.push(SingleExpression::from_pest(&mut pairs)?);
        }

        Ok(Self { elements })
    }
}
