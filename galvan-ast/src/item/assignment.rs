use from_pest::pest::iterators::Pairs;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;
use typeunion::type_union;

use super::*;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::assignment))]
pub struct Assignment {
    pub target: AssignmentTarget,
    pub operator: AssignmentOperator,
    pub expression: Expression,
}

#[type_union]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::assignment_target))]
pub type AssignmentTarget = Ident + MemberFieldAccess;

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
