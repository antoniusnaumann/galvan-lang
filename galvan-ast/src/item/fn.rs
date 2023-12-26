use from_pest::pest::iterators::Pairs;
use from_pest::{ConversionError, FromPest, Void};
use galvan_pest::Rule;

use super::*;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::function))]
pub struct FnDecl {
    pub signature: FnSignature,
    pub block: Block,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::fn_signature))]
pub struct FnSignature {
    // pub asyncness: Async,
    // pub constness: Const,
    pub visibility: Visibility,
    pub identifier: Ident,
    pub parameters: ParamList,
    pub return_type: Option<TypeElement>,
}

impl FnSignature {
    pub fn new(
        mods: Modifiers,
        ident: Ident,
        parameters: ParamList,
        return_type: Option<TypeElement>,
    ) -> Self {
        FnSignature {
            // asyncness: mods.asyncness,
            // constness: mods.constness,
            visibility: mods.visibility,
            identifier: ident,
            parameters,
            return_type,
        }
    }
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::param_list))]
pub struct ParamList {
    pub params: Vec<Param>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::param))]
pub struct Param {
    pub decl_modifier: DeclModifier,
    pub identifier: Ident,
    pub param_type: TypeElement,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DeclModifier {
    Let,
    Mut,
    Ref,
    Inherited,
}

impl FromPest<'_> for DeclModifier {
    type Rule = Rule;
    type FatalError = Void;

    fn from_pest(pairs: &mut Pairs<Self::Rule>) -> Result<Self, ConversionError<Self::FatalError>> {
        let no_match = || Err(ConversionError::NoMatch);

        let pair = pairs.next().unwrap();
        if pair.as_rule() != Rule::declaration_modifier {
            return no_match();
        }

        let Some(pair) = pair.into_inner().next() else {
            return no_match();
        };

        match pair.as_rule() {
            Rule::let_keyword => Ok(DeclModifier::Let),
            Rule::mut_keyword => Ok(DeclModifier::Mut),
            Rule::ref_keyword => Ok(DeclModifier::Ref),
            Rule::inherited => Ok(DeclModifier::Inherited),
            _ => no_match(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::body))]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::statement))]
pub struct Statement {}
