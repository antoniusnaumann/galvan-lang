use crate::*;

pub struct FnDecl {
    pub visibility_modifier: Visibility,
    pub signature: FnSignatur,
    pub block: Block,
}

pub struct FnSignatur {
    pub asyncness: Async,
    pub constness: Const,
    pub receiver: Option<ReceiverType>,
    pub identifier: Ident,
    pub parameter: ParamList,
    pub return_type: ReturnType,
}

pub struct ParamList {
    pub params: Vec<Param>,
}

pub struct Param {
    pub identifier: Ident,
    pub param_type: ParamType,
}

pub struct Block {
    pub statements: Vec<Statement>,
}

pub struct Statement {}
