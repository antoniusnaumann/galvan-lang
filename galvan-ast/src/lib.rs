#[macro_use]
extern crate core;

use std::ops::Deref;

use galvan_files::Source;

mod item;

pub use item::*;

mod result;
use result::Result;
pub use result::{AstError, AstResult};

#[derive(Debug, PartialEq, Eq)]
pub struct Ast {
    pub toplevel: Vec<RootItem>,
    pub source: Source,
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

pub trait SegmentAst {
    fn segmented(self) -> Result<SegmentedAsts>;
}

impl SegmentAst for Ast {
    fn segmented(self) -> Result<SegmentedAsts> {
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut tests = Vec::new();
        let mut main = None;

        for item in self.toplevel {
            match item {
                RootItem::Type(item) => types.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
                RootItem::Fn(item) => functions.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
                RootItem::Test(item) => tests.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
                RootItem::Main(item) => {
                    if main.is_some() {
                        return Err(AstError::DuplicateMain);
                    }

                    main = Some(ToplevelItem {
                        item,
                        source: self.source.clone(),
                    })
                }
            }
        }

        Ok(SegmentedAsts {
            types,
            functions,
            tests,
            main,
        })
    }
}

impl SegmentAst for Vec<Ast> {
    fn segmented(self) -> Result<SegmentedAsts> {
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut tests = Vec::new();
        let mut main = None;
        let segmented = self.into_iter().map(SegmentAst::segmented);

        for ast in segmented {
            let ast = ast?;
            types.extend(ast.types);
            functions.extend(ast.functions);
            tests.extend(ast.tests);
            if let Some(main_decl) = ast.main {
                if main.is_some() {
                    return Err(AstError::DuplicateMain);
                }

                main = Some(main_decl);
            }
        }

        Ok(SegmentedAsts {
            types,
            functions,
            tests,
            main,
        })
    }
}
