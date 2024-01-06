use crate::{
    Block, ConstructorCall, Expression, FunctionCall, Ident, MemberFieldAccess, MemberFunctionCall,
    TypeElement,
};
use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;
use typeunion::type_union;

#[derive(Debug, PartialEq, Eq)]
pub struct Closure {
    pub arguments: Vec<ClosureArgument>,
    pub block: Block,
}

impl FromPest<'_> for Closure {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let Some(pair) = pairs.next() else {
            return Err(NoMatch);
        };
        if pair.as_rule() != Rule::closure && pair.as_rule() != Rule::trailing_closure {
            return Err(NoMatch);
        }

        let mut pairs = pair.into_inner();
        let arguments = Vec::<ClosureArgument>::from_pest(&mut pairs)?;
        let block = Block::from_pest(&mut pairs)?;

        Ok(Self { arguments, block })
    }
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::closure_argument))]
pub struct ClosureArgument {
    pub ident: Ident,
    pub ty: Option<TypeElement>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ElseExpression {
    pub receiver: Box<Expression>,
    pub block: Block,
}

#[type_union(super = Expression)]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::allowed_before_else_expression))]
type AllowedBeforeElseExpression =
    FunctionCall + ConstructorCall + MemberFunctionCall + MemberFieldAccess + Ident;

impl FromPest<'_> for ElseExpression {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let Some(pair) = pairs.next() else {
            return Err(NoMatch);
        };
        if pair.as_rule() != Rule::else_expression {
            return Err(NoMatch);
        }

        let mut pairs = pair.into_inner();
        let receiver = Box::new(AllowedBeforeElseExpression::from_pest(&mut pairs)?.into());
        let block = Block::from_pest(&mut pairs)?;

        Ok(Self { receiver, block })
    }
}
