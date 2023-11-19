use super::*;

#[derive(Debug, /* FromPest */)]
// #[pest_ast(rule(Rule::function))]
pub struct FnDecl {
    pub signature: FnSignature,
    pub block: Block,
}

#[derive(Debug)]
pub struct FnSignature {
    pub asyncness: Async,
    pub constness: Const,
    pub visibility: Visibility,
    pub receiver: Option<ReceiverType>,
    pub identifier: Ident,
    pub parameters: ParamList,
    pub return_type: Option<ReturnType>,
}

impl FnSignature {
    pub fn new(
        mods: Modifiers,
        receiver: Option<ReceiverType>,
        ident: Ident,
        parameters: ParamList,
        return_type: Option<ReturnType>,
    ) -> Self {
        FnSignature {
            asyncness: mods.asyncness,
            constness: mods.constness,
            visibility: mods.visibility,
            receiver,
            identifier: ident,
            parameters,
            return_type,
        }
    }
}

#[derive(Debug)]
pub struct ParamList {
    pub params: Vec<Param>,
}

#[derive(Debug)]
pub struct Param {
    pub identifier: Ident,
    pub param_type: ParamType,
}

#[derive(Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub struct Statement {}
