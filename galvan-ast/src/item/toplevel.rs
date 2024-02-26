use derive_more::From;

use super::{FnDecl, MainDecl, TestDecl, TypeDecl};

#[derive(Debug, PartialEq, Eq, From)]
pub enum RootItem {
    Fn(FnDecl),
    Type(TypeDecl),
    Main(MainDecl),
    Test(TestDecl),
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
// impl RootItemMarker for TaskDecl {}
