use crate::*;

#[derive(Debug)]
pub struct FnDecl {
    pub visibility_modifier: Visibility,
    pub signature: FnSignatur,
    pub block: Block,
}

#[derive(Debug)]
pub struct FnSignatur {
    pub asyncness: Async,
    pub constness: Const,
    pub receiver: Option<ReceiverType>,
    pub identifier: Ident,
    pub parameter: ParamList,
    pub return_type: ReturnType,
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
