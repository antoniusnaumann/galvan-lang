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
    pub identifier: Ident,
    pub param_type: TypeElement,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::body))]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::statement))]
pub struct Statement {}
