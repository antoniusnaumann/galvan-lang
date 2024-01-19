use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;
use typeunion::type_union;

use super::*;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::assignment))]
pub struct Assignment {
    pub target: AssignmentTarget,
    pub operator: AssignmentOperator,
    pub expression: TopExpression,
}

#[type_union]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::assignment_target))]
pub type AssignmentTarget = Ident + MemberFieldAccess;

#[type_union]
#[derive(Debug, PartialEq, Eq)]
pub type TopExpression = Expression + ElseExpression;

impl FromPest<'_> for TopExpression {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.next().ok_or(NoMatch)?;
        if pair.as_rule() != Rule::top_expression {
            return Err(NoMatch);
        }

        let mut pairs = pair.into_inner();
        match pairs.peek().ok_or(NoMatch)?.as_rule() {
            Rule::expression => {
                let expression = Expression::from_pest(&mut pairs)?;
                Ok(expression.into())
            }
            Rule::else_expression => {
                let else_expression = ElseExpression::from_pest(&mut pairs)?;
                Ok(else_expression.into())
            }
            Rule::trailing_closure_call => {
                let function_call = FunctionCall::from_pest(&mut pairs)?;
                Ok(
                    Expression::from(SimpleExpression::from(SingleExpression::from(
                        function_call,
                    )))
                    .into(),
                )
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AssignmentOperator {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    PowAssign,
}

impl FromPest<'_> for AssignmentOperator {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let Some(pair) = pairs.next() else {
            return Err(ConversionError::NoMatch);
        };
        let Some(pair) = pair.into_inner().next() else {
            return Err(ConversionError::NoMatch);
        };

        match pair.as_rule() {
            Rule::assign => Ok(Self::Assign),
            Rule::add_assign => Ok(Self::AddAssign),
            Rule::sub_assign => Ok(Self::SubAssign),
            Rule::mul_assign => Ok(Self::MulAssign),
            Rule::div_assign => Ok(Self::DivAssign),
            Rule::rem_assign => Ok(Self::RemAssign),
            Rule::pow_assign => Ok(Self::PowAssign),
            _ => unreachable!(),
        }
    }
}
