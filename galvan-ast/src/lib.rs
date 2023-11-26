#[macro_use]
extern crate pest_ast;
extern crate core;

use derive_more::From;
use from_pest::FromPest;

use galvan_pest::*;
use galvan_files::Source;

mod item;

pub use item::*;

mod result;
pub use result::{AstError, AstResult};
use result::Result;
use crate::pest_adapter::IntoPestAst;

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

pub trait IntoAst {
    fn try_into_ast(self) -> AstResult;
}

impl IntoAst for Source {
    fn try_into_ast(self) -> AstResult {
        let parsed = parse_source(&self)?;
        parsed.try_into_ast().map(|ast| ast.with_source(self))
    }
}