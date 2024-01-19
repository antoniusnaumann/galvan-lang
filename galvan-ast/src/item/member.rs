use crate::{FunctionCall, Ident, SingleExpression};
use from_pest::pest::iterators::Pairs;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;

#[derive(Debug, PartialEq, Eq)]
pub struct MemberChainBase {
    pub base: Vec<SingleExpression>,
}

impl FromPest<'_> for MemberChainBase {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(ConversionError::NoMatch)?;
        if pair.as_rule() != Rule::member_chain_base {
            return Err(ConversionError::NoMatch);
        }

        let mut pairs = pair.into_inner();
        let mut base = Vec::new();
        while let Some(pair) = pairs.peek() {
            let rule = pair.as_rule();
            match rule {
                Rule::single_expression => {
                    base.push(SingleExpression::from_pest(&mut pairs)?);
                }
                Rule::strict_trailing_closure_call => {
                    base.push(FunctionCall::from_pest(&mut pairs)?.into());
                }
                _ => Err(ConversionError::NoMatch)?,
            }
        }

        Ok(Self { base })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MemberFunctionCall {
    // TODO: Parse tuples of single expression and access operators
    pub base: MemberChainBase,
    pub call: FunctionCall,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MemberFieldAccess {
    pub base: MemberChainBase,
    pub field: Ident,
}

impl FromPest<'_> for MemberFunctionCall {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let base = MemberChainBase::from_pest(pairs)?;
        let call = FunctionCall::from_pest(pairs)?;
        Ok(Self { base, call })
    }
}

impl FromPest<'_> for MemberFieldAccess {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let base = MemberChainBase::from_pest(pairs)?;
        let field = Ident::from_pest(pairs)?;
        Ok(Self { base, field })
    }
}
