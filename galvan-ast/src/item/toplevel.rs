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

trait Seal {}
pub trait RootItemMarker: Seal {}

impl Seal for FnDecl {}

impl RootItemMarker for FnDecl {}

impl Seal for TypeDecl {}

impl RootItemMarker for TypeDecl {}

impl Seal for MainDecl {}

impl RootItemMarker for MainDecl {}

impl Seal for TestDecl {}

impl RootItemMarker for TestDecl {}
// impl RootItemMarker for TaskDecl {}
