use crate::{FunctionCall, Ident, SingleExpression};
use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;

#[derive(Debug, PartialEq, Eq)]
pub struct MemberChain {
    pub elements: Vec<SingleExpression>,
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
        while let Some(pair) = pairs.peek() {
            // println!("Member chain element: {:#?}", pair);
            // println!("Member chain: {:#?}", elements);
            elements.push(SingleExpression::from_pest(&mut pairs)?);
        }

        Ok(Self { elements })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MemberFunctionCall {
    // TODO: Parse tuples of single expression and access operators
    pub base: Vec<SingleExpression>,
    pub call: FunctionCall,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MemberFieldAccess {
    pub base: Vec<SingleExpression>,
    pub field: Ident,
}

impl FromPest<'_> for MemberFunctionCall {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let mut elements = MemberChain::from_pest(pairs)?.elements;
        let call = match elements.pop() {
            Some(SingleExpression::FunctionCall(f)) => f,
            _ => return Err(NoMatch),
        };

        Ok(Self {
            base: elements,
            call,
        })
    }
}

impl FromPest<'_> for MemberFieldAccess {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let mut elements = MemberChain::from_pest(pairs)?.elements;
        let field = match elements.pop() {
            Some(SingleExpression::Ident(i)) => i,
            _ => return Err(NoMatch),
        };
        Ok(Self {
            base: elements,
            field,
        })
    }
}
