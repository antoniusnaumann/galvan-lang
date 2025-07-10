#[macro_use]
extern crate core;

use std::{fmt::Display, ops::Deref};

use galvan_files::Source;

mod item;

pub use item::*;
use itertools::Itertools;

#[derive(Debug, PartialEq, Eq)]
pub struct Ast {
    pub toplevel: Vec<RootItem>,
    pub source: Source,
}

impl Ast {
    pub fn with_source(self, source: Source) -> Ast {
        if self.source != Source::Missing {
            panic!("Attempting to set a source to an AST that already had a source!");
        }

        Ast {
            toplevel: self.toplevel,
            source,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ToplevelItem<R: RootItemMarker> {
    pub item: R,
    pub source: Source,
    // pub span: Span,
}

impl<R> Deref for ToplevelItem<R>
where
    R: RootItemMarker,
{
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SegmentedAsts {
    pub types: Vec<ToplevelItem<TypeDecl>>,
    pub functions: Vec<ToplevelItem<FnDecl>>,
    pub tests: Vec<ToplevelItem<TestDecl>>,
    pub main: Option<ToplevelItem<MainDecl>>,
    // pub other: Vec<ToplevelItem<CustomTaskDecl>>
}

pub trait PrintAst {
    fn print_ast(&self, indent: usize) -> String;
}

pub trait AstNode {
    fn span(&self) -> Span;
    fn print(&self, indent: usize) -> String;
}

impl PrintAst for bool {
    fn print_ast(&self, indent: usize) -> String {
        let indent_str = " ".repeat(indent);
        format!("{indent_str}{self}\n")
    }
}

impl<T> PrintAst for Vec<T>
where
    T: PrintAst,
{
    fn print_ast(&self, indent: usize) -> String {
        self.iter().map(|i| i.print_ast(indent)).join("\n")
    }
}

impl<T> PrintAst for Option<T>
where
    T: PrintAst,
{
    fn print_ast(&self, indent: usize) -> String {
        self.iter().map(|i| i.print_ast(indent)).join("\n")
    }
}

impl<T> PrintAst for T
where
    T: AstNode,
{
    fn print_ast(&self, indent: usize) -> String {
        AstNode::print(self, indent)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub range: (usize, usize),
    /// Start as Row, Column position
    pub start: Point,
    /// End as Row, Column position
    pub end: Point,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub row: usize,
    pub col: usize,
}
