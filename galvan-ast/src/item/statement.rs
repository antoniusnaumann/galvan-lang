use super::*;
use crate::item::closure::Closure;
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
#[pest_ast(rule(Rule::expression))]
pub type Expression = ElseExpression
    + Closure
    + LogicalOperation
    + ComparisonOperation
    + CollectionOperation
    + ArithmeticOperation
    + CollectionLiteral
    + FunctionCall
    + ConstructorCall
    + MemberFunctionCall
    + MemberFieldAccess
    + BooleanLiteral
    + StringLiteral
    + NumberLiteral
    + Ident;

#[type_union(super = Expression)]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::allowed_in_logical))]
pub(crate) type AllowedInLogical = ComparisonOperation
    + CollectionOperation
    + ArithmeticOperation
    + CollectionLiteral
    + FunctionCall
    + ConstructorCall
    + MemberFunctionCall
    + MemberFieldAccess
    + BooleanLiteral
    + StringLiteral
    + NumberLiteral
    + Ident;

#[type_union(super = Expression)]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::allowed_in_comparison))]
pub(crate) type AllowedInComparison = CollectionOperation
    + ArithmeticOperation
    + CollectionLiteral
    + FunctionCall
    + ConstructorCall
    + MemberFunctionCall
    + MemberFieldAccess
    + BooleanLiteral
    + StringLiteral
    + NumberLiteral
    + Ident;

#[type_union(super = Expression)]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::allowed_in_collection))]
pub(crate) type AllowedInCollection = ArithmeticOperation
    + CollectionLiteral
    + FunctionCall
    + ConstructorCall
    + MemberFunctionCall
    + MemberFieldAccess
    + BooleanLiteral
    + StringLiteral
    + NumberLiteral
    + Ident;

#[type_union(super = Expression)]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::allowed_in_arithmetic))]
pub(crate) type AllowedInArithmetic = CollectionLiteral
    + FunctionCall
    + ConstructorCall
    + MemberFunctionCall
    + MemberFieldAccess
    + BooleanLiteral
    + StringLiteral
    + NumberLiteral
    + Ident;
