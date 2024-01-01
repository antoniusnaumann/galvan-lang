use crate::{DeclModifier, Expression, Ident, TypeIdent};
use derive_more::From;
use galvan_pest::Rule;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::function_call))]
pub struct FunctionCall {
    pub identifier: Ident,
    pub arguments: Vec<FunctionCallArg>,
}

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::function_call_arg))]
pub struct FunctionCallArg {
    pub modifier: DeclModifier,
    pub expression: Expression,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::member_function_call))]
pub struct MemberFunctionCall {
    pub receiver: Ident,
    pub identifier: Ident,
    pub arguments: Vec<FunctionCallArg>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::member_field_access))]
pub struct MemberFieldAccess {
    pub receiver: Ident,
    pub identifier: Ident,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::constructor_call))]
pub struct ConstructorCall {
    pub identifier: TypeIdent,
    pub arguments: Vec<ConstructorCallArg>,
}

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::constructor_call_arg))]
pub struct ConstructorCallArg {
    pub ident: Ident,
    pub expression: Expression,
}
