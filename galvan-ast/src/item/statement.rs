use super::*;
use galvan_pest::Rule;
use typeunion::type_union;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::body))]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[type_union]
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::statement))]
pub type Statement = Assignment + Expression + Declaration;

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
pub type Expression = LogicalOperation
    + ComparisonOperation
    + CollectionOperation
    + ArithmeticOperation
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
pub(crate) type AllowedInArithmetic = FunctionCall
    + ConstructorCall
    + MemberFunctionCall
    + MemberFieldAccess
    + BooleanLiteral
    + StringLiteral
    + NumberLiteral
    + Ident;
