#[macro_use]
extern crate core;

use std::ops::Deref;

use galvan_files::Source;

mod item;

pub use item::*;

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


