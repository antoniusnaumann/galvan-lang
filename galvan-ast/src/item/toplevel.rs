use derive_more::From;
use galvan_ast_macro::AstNode;

use super::{Body, FnDecl, Ident, StringLiteral, TypeDecl};
use crate::{AstNode, PrintAst, Span};

#[derive(Debug, PartialEq, Eq, From)]
pub enum RootItem {
    Fn(FnDecl),
    Type(TypeDecl),
    Main(MainDecl),
    Test(TestDecl),
    // CustomTask(TaskDecl),
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct MainDecl {
    pub body: Body,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TestDecl {
    pub name: Option<StringLiteral>,
    pub body: Body,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TaskDecl {
    pub ident: Ident,
    // name: Option<String>,
    pub body: Body,
}

mod private {
    pub trait Seal {}
}

pub trait RootItemMarker: private::Seal {}

impl private::Seal for FnDecl {}

impl RootItemMarker for FnDecl {}

impl private::Seal for TypeDecl {}

impl RootItemMarker for TypeDecl {}

impl private::Seal for MainDecl {}

impl RootItemMarker for MainDecl {}

impl private::Seal for TestDecl {}

impl RootItemMarker for TestDecl {}
// impl RootItemMarker for TaskDecl {}
