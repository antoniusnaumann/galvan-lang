use super::*;
use galvan_pest::Rule;

#[derive(Debug, Clone, PartialEq, FromPest)]
#[pest_ast(rule(Rule::infix_operator))]
pub enum InfixOperator {
    Arithmetic(ArithmeticOperator),
    // Bitwise(BitwiseOperator),
    Collection(CollectionOperator),
    Comparison(ComparisonOperator),
    Logical(LogicalOperator),
    CustomInfix(CustomInfixOperator),
}

#[derive(Debug, Clone, PartialEq, FromPest)]
#[pest_ast(rule(Rule::arithmetic_operator))]
pub enum ArithmeticOperator {
    #[pest_ast(atomic)]
    Plus,
    #[pest_ast(atomic)]
    Minus,
    #[pest_ast(atomic)]
    Multiply,
    #[pest_ast(atomic)]
    Divide,
    #[pest_ast(atomic)]
    Remainder,
    #[pest_ast(atomic)]
    Power,
}

#[derive(Debug, Clone, PartialEq, FromPest)]
#[pest_ast(rule(Rule::collection_operator))]
pub enum CollectionOperator {
    #[pest_ast(atomic)]
    Concat,
    #[pest_ast(atomic)]
    Remove,
    #[pest_ast(atomic)]
    Contains,
}

#[derive(Debug, Clone, PartialEq, FromPest)]
#[pest_ast(rule(Rule::comparison_operator))]
pub enum ComparisonOperator {
    #[pest_ast(atomic)]
    Equal,
    #[pest_ast(atomic)]
    NotEqual,
    #[pest_ast(atomic)]
    Less,
    #[pest_ast(atomic)]
    LessEqual,
    #[pest_ast(atomic)]
    Greater,
    #[pest_ast(atomic)]
    GreaterEqual,
    #[pest_ast(atomic)]
    Identical,
    #[pest_ast(atomic)]
    NotIdentical,
}

#[derive(Debug, Clone, PartialEq, FromPest)]
#[pest_ast(rule(Rule::logical_infix_operator))]
pub enum LogicalOperator {
    #[pest_ast(atomic)]
    And,
    #[pest_ast(atomic)]
    Or,
    #[pest_ast(atomic)]
    Xor,
}

#[derive(Debug, Clone, PartialEq, FromPest)]
#[pest_ast(rule(Rule::custom_infix_operator))]
pub struct CustomInfixOperator(#[pest_ast(outer(with(string)))] String);
