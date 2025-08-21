use crate::{DeclModifier, EnumAccess, Expression, Ident, PrintAst, TypeIdent};
use derive_more::From;
use galvan_ast_macro::PrintAst;

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct FunctionCall {
    pub identifier: Ident,
    pub arguments: Vec<FunctionCallArg>,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct FunctionCallArg {
    pub modifier: Option<DeclModifier>,
    pub expression: Expression,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct ConstructorCall {
    pub identifier: TypeIdent,
    pub arguments: Vec<ConstructorCallArg>,
}

#[derive(Clone, Debug, PartialEq, Eq, From, PrintAst)]
pub struct ConstructorCallArg {
    pub ident: Ident,
    pub expression: Expression,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct EnumConstructor {
    pub enum_access: EnumAccess,
    pub arguments: Vec<EnumConstructorArg>,
}

#[derive(Clone, Debug, PartialEq, Eq, PrintAst)]
pub struct EnumConstructorArg {
    pub field_name: Option<Ident>,  // None for anonymous args
    pub modifier: Option<DeclModifier>,
    pub expression: Expression,
}
