use crate::{
    Block, Body, Expression, FunctionCall, Ident, SingleExpression, TopExpression, TypeElement,
};
use from_pest::pest::iterators::Pairs;
use from_pest::ConversionError::NoMatch;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;

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
        let rule = pair.as_rule();
        if rule != Rule::closure && rule != Rule::trailing_closure {
            return Err(NoMatch);
        }

        let mut pairs = pair.into_inner();
        let arguments = Vec::<ClosureArgument>::from_pest(&mut pairs)?;

        let block = match rule {
            Rule::closure => Block::from_pest(&mut pairs).or_else(|_| {
                Expression::from_pest(&mut pairs).map(|e| Block {
                    body: Body {
                        statements: vec![TopExpression::from(e).into()],
                    },
                })
            })?,
            Rule::trailing_closure => Block::from_pest(&mut pairs)?,
            _ => unreachable!(),
        };

        Ok(Self { arguments, block })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::closure_argument))]
pub struct ClosureArgument {
    pub ident: Ident,
    pub ty: Option<TypeElement>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ElseExpression {
    pub receiver: Box<SingleExpression>,
    pub block: Block,
}

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
        // println!("Else-Expression: \n{:#?}", pairs);
        let receiver_pair = pairs.peek().ok_or(NoMatch)?;

        match receiver_pair.as_rule() {
            Rule::single_expression => {
                let receiver = Box::new(SingleExpression::from_pest(&mut pairs)?);
                let block = Block::from_pest(&mut pairs)?;
                Ok(Self { receiver, block })
            }
            Rule::trailing_closure_call => {
                let receiver = Box::new(FunctionCall::from_pest(&mut pairs)?.into());
                let block = Block::from_pest(&mut pairs)?;
                Ok(Self { receiver, block })
            }
            _ => Err(NoMatch),
        }
    }
}
