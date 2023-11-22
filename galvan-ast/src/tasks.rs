use crate::literal::StringLiteral;
use super::*;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::main))]
pub struct MainDecl {
    // TODO: body
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::test))]
pub struct TestDecl {
    name: Option<StringLiteral>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::task))]
pub struct TaskDecl {
    ident: Ident,
    // name: Option<String>,
    body: Body,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::body))]
pub struct Body {
    // statements: Vec<Statement>,
}
