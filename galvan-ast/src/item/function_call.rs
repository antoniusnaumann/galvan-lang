use crate::{DeclModifier, Expression, Ident, TypeIdent};
use derive_more::From;

#[derive(Debug, PartialEq, Eq)]
pub struct FunctionCall {
    pub identifier: Ident,
    pub arguments: Vec<FunctionCallArg>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FunctionCallArg {
    pub modifier: Option<DeclModifier>,
    pub expression: Expression,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConstructorCall {
    pub identifier: TypeIdent,
    pub arguments: Vec<ConstructorCallArg>,
}

#[derive(Debug, PartialEq, Eq, From)]
pub struct ConstructorCallArg {
    pub ident: Ident,
    pub expression: Expression,
}
