#[macro_use]
extern crate pest_ast;
extern crate core;

use derive_more::From;
use from_pest::FromPest;
use std::ops::Deref;

use galvan_files::Source;
use galvan_pest::*;

mod item;

pub use item::*;

mod result;
use crate::pest_adapter::IntoPestAst;
use result::Result;
pub use result::{AstError, AstResult};

pub mod pest_adapter {
    use super::*;
    #[derive(Debug, PartialEq, Eq, From, FromPest)]
    #[pest_ast(rule(Rule::source))]
    pub struct PestAst {
        pub toplevel: Vec<RootItem>,
        _eoi: _EOI,
    }

    pub trait IntoPestAst {
        fn try_into_ast(self) -> Result<PestAst>;
    }

    #[derive(Debug, Default, PartialEq, Eq, FromPest)]
    #[pest_ast(rule(Rule::EOI))]
    struct _EOI;

    impl IntoPestAst for ParserNodes<'_> {
        fn try_into_ast(mut self) -> Result<PestAst> {
            Ok(PestAst::from_pest(&mut self)?)
        }
    }

    impl PestAst {
        pub fn new(toplevel: Vec<RootItem>) -> Self {
            PestAst {
                toplevel,
                _eoi: _EOI,
            }
        }

        pub fn with_source(self, source: Source) -> Ast {
            Ast {
                toplevel: self.toplevel,
                source,
            }
        }
    }

    impl From<RootItem> for PestAst {
        fn from(item: RootItem) -> Self {
            PestAst::new(vec![item])
        }
    }
}
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

pub trait IntoAst {
    fn try_into_ast(self) -> AstResult;
}

impl IntoAst for Source {
    fn try_into_ast(self) -> AstResult {
        let parsed = parse_source(&self)?;
        parsed.try_into_ast().map(|ast| ast.with_source(self))
    }
}
