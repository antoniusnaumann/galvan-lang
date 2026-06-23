use derive_more::From;
use galvan_ast_macro::AstNode;

use super::{Body, FnDecl, Ident, Param, ParamList, StringLiteral, TypeDecl};
use crate::{AstNode, PrintAst, Span};

#[derive(Debug, PartialEq, Eq)]
pub struct MainDecl {
    pub kind: MainKind,
    pub body: Body,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MainKind {
    Function { argument: Option<Param> },
    Command(CmdSignature),
}

#[derive(Debug, PartialEq, Eq)]
pub struct TestDecl {
    pub name: Option<StringLiteral>,
    pub body: Body,
}

#[derive(Debug, PartialEq, Eq, AstNode)]
pub struct CmdDecl {
    pub signature: CmdSignature,
    pub body: Body,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct CmdSignature {
    pub identifier: Ident,
    pub parameters: ParamList,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TaskDecl {
    pub ident: Ident,
    // name: Option<String>,
    pub body: Body,
}

#[derive(Debug, PartialEq, Eq, From)]
pub enum RootItem {
    Fn(FnDecl),
    Type(TypeDecl),
    Test(TestDecl),
    Cmd(CmdDecl),
    // CustomTask(TaskDecl),
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

impl private::Seal for CmdDecl {}

impl RootItemMarker for CmdDecl {}
// impl RootItemMarker for TaskDecl {}
