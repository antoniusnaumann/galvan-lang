use derive_more::From;
use galvan_pest::Rule;

use super::{FnDecl, MainDecl, TaskDecl, TestDecl, TypeDecl};

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::toplevel))]
pub enum RootItem {
    Fn(FnDecl),
    Type(TypeDecl),
    Main(MainDecl),
    Test(TestDecl),
    // CustomTask(TaskDecl),
}
