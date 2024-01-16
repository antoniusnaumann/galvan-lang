use super::*;
use crate::item::closure::Closure;
use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;
use typeunion::type_union;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::body))]
pub struct Body {
    pub statements: Vec<Statement>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::block))]
pub struct Block {
    pub body: Body,
}

#[type_union]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::statement))]
pub type Statement = Assignment + Expression + Declaration + Block;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::declaration))]
pub struct Declaration {
    pub decl_modifier: DeclModifier,
    pub identifier: Ident,
    pub type_annotation: Option<TypeElement>,
    pub expression: Option<Expression>,
}

#[type_union]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::literal))]
pub type Literal = BooleanLiteral + StringLiteral + NumberLiteral;

#[type_union]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::expression))]
pub type Expression =
    OperatorChain + MemberFunctionCall + MemberFieldAccess + SingleExpression + Closure;

#[type_union(super = Expression)]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::simple_expression))]
pub type SimpleExpression = MemberFunctionCall + MemberFieldAccess + SingleExpression;

#[type_union]
#[derive(Debug, PartialEq, Eq)]
pub type SingleExpression = CollectionLiteral + FunctionCall + ConstructorCall + Literal + Ident;

impl FromPest<'_> for SingleExpression {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(
        pairs: &mut Pairs<'_, Self::Rule>,
    ) -> Result<Self, ConversionError<Self::FatalError>> {
        let pair = pairs.peek().ok_or(NoMatch)?;
        let rule = pair.as_rule();
        match rule {
            Rule::trailing_closure_call => {
                let function_call = FunctionCall::from_pest(pairs)?;
                Ok(function_call.into())
            }
            Rule::single_expression => {
                pairs.next();
                let mut pairs = pair.into_inner();
                let pair = pairs.next().ok_or(NoMatch)?;
                let rule = pair.as_rule();
                match rule {
                    Rule::collection_literal => {
                        let collection_literal = CollectionLiteral::from_pest(&mut pairs)?;
                        Ok(collection_literal.into())
                    }
                    Rule::function_call => {
                        let function_call = FunctionCall::from_pest(&mut pairs)?;
                        Ok(function_call.into())
                    }
                    Rule::constructor_call => {
                        let constructor_call = ConstructorCall::from_pest(&mut pairs)?;
                        Ok(constructor_call.into())
                    }
                    Rule::literal => {
                        let literal = Literal::from_pest(&mut pairs)?;
                        Ok(literal.into())
                    }
                    Rule::ident => {
                        let ident = Ident::from_pest(&mut pairs)?;
                        Ok(ident.into())
                    }
                    _ => Err(NoMatch),
                }
            }
            _ => Err(NoMatch),
        }
    }
}
