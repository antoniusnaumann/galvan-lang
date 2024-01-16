use crate::{FunctionCall, Ident, SingleExpression};
use from_pest::pest::iterators::Pairs;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::member_chain_base))]
pub struct MemberChainBase {
    pub base: Vec<SingleExpression>,
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
