use galvan_pest::Rule;

use super::{Block, Ident, StringLiteral};

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::main))]
pub struct MainDecl {
    pub body: Block,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::test))]
pub struct TestDecl {
    pub name: Option<StringLiteral>,
    pub body: Block,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::task))]
pub struct TaskDecl {
    pub ident: Ident,
    // name: Option<String>,
    pub body: Block,
}
