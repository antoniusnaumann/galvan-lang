use crate::item::AllowedInMemberCall;
use crate::{
    ArithmeticOperation, BooleanLiteral, Closure, CollectionLiteral, CollectionOperation,
    ComparisonOperation, DeclModifier, Expression, Ident, LogicalOperation, NumberLiteral,
    StringLiteral, TypeIdent,
};
use derive_more::From;
use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;
use typeunion::type_union;

#[derive(Debug, PartialEq, Eq)]
pub struct FunctionCall {
    pub identifier: Ident,
    pub arguments: Vec<FunctionCallArg>,
}

impl FromPest<'_> for FunctionCall {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let Some(pair) = pairs.next() else {
            return Err(NoMatch);
        };
        let rule = pair.as_rule();
        match rule {
            Rule::function_call | Rule::trailing_closure_call => {
                let mut pairs = pair.into_inner();
                let identifier = Ident::from_pest(&mut pairs)?;

                let arguments = if rule == Rule::function_call {
                    Vec::<FunctionCallArg>::from_pest(&mut pairs)?
                } else {
                    let arguments = Vec::<TrailingClosureCallArg>::from_pest(&mut pairs)?;
                    let mut arguments = arguments
                        .into_iter()
                        .map(|arg| FunctionCallArg {
                            modifier: arg.modifier,
                            expression: arg.expression.into(),
                        })
                        .collect::<Vec<_>>();
                    if let Ok(closure) = Closure::from_pest(&mut pairs) {
                        arguments.push(FunctionCallArg {
                            modifier: None,
                            expression: closure.into(),
                        });
                    }
                    arguments
                };

                Ok(Self {
                    identifier,
                    arguments,
                })
            }
            _ => Err(NoMatch),
        }
    }
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::function_call_arg))]
pub struct FunctionCallArg {
    pub modifier: Option<DeclModifier>,
    pub expression: Expression,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::trailing_closure_call_arg))]
struct TrailingClosureCallArg {
    modifier: Option<DeclModifier>,
    expression: AllowedInTrailingClosureCall,
}

#[type_union(super = Expression)]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::allowed_in_trailing_closure_call))]
type AllowedInTrailingClosureCall = LogicalOperation
    + ComparisonOperation
    + CollectionOperation
    + ArithmeticOperation
    + CollectionLiteral
    + FunctionCall
    + ConstructorCall
    + MemberFunctionCall
    + MemberFieldAccess
    + BooleanLiteral
    + StringLiteral
    + NumberLiteral
    + Ident;

#[derive(Debug, PartialEq, Eq)]
pub struct MemberFunctionCall {
    pub receiver: Vec<Expression>,
    pub identifier: Ident,
    pub arguments: Vec<FunctionCallArg>,
}

impl FromPest<'_> for MemberFunctionCall {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let Some(pair) = pairs.next() else {
            return Err(NoMatch);
        };

        if pair.as_rule() != Rule::member_function_call {
            return Err(NoMatch);
        }

        let mut pairs = pair.into_inner();

        let (receiver, identifier) = parse_member_chain(&mut pairs)?;
        let arguments = Vec::<FunctionCallArg>::from_pest(&mut pairs)?;

        Ok(Self {
            receiver,
            identifier,
            arguments,
        })
    }
}

fn parse_member_chain(
    pairs: &mut Pairs<'_, Rule>,
) -> Result<(Vec<Expression>, Ident), ConversionError<Void>> {
    let receiver = Vec::<AllowedInMemberCall>::from_pest(pairs)?
        .into_iter()
        .map(|expr| expr.into())
        .collect::<Vec<_>>();
    let identifier = Ident::from_pest(pairs)?;

    Ok((receiver, identifier))
}

#[derive(Debug, PartialEq, Eq)]
pub struct MemberFieldAccess {
    pub receiver: Vec<Expression>,
    pub identifier: Ident,
}

impl FromPest<'_> for MemberFieldAccess {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let Some(pair) = pairs.next() else {
            return Err(NoMatch);
        };

        if pair.as_rule() != Rule::member_field_access {
            return Err(NoMatch);
        }

        let mut pairs = pair.into_inner();

        let (receiver, identifier) = parse_member_chain(&mut pairs)?;

        Ok(Self {
            receiver,
            identifier,
        })
    }
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::constructor_call))]
pub struct ConstructorCall {
    pub identifier: TypeIdent,
    pub arguments: Vec<ConstructorCallArg>,
}

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::constructor_call_arg))]
pub struct ConstructorCallArg {
    pub ident: Ident,
    pub expression: Expression,
}
