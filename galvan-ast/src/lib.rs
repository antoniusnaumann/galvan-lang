#[macro_use]
extern crate pest_ast;

use derive_more::From;
use from_pest::FromPest;

use galvan_pest::*;
pub use galvan_pest::{Source};

mod item;

pub use item::*;

mod result;
pub use result::*;

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::source))]
pub struct Ast {
    pub toplevel: Vec<RootItem>,
    _eoi: _EOI,
}

impl Ast {
    pub fn new(toplevel: Vec<RootItem>) -> Self {
        Ast {
            toplevel,
            _eoi: _EOI,
        }
    }
}

impl From<RootItem> for Ast {
    fn from(item: RootItem) -> Self {
        Ast::new(vec![item])
    }
}

pub trait IntoAst {
    fn try_into_ast(self) -> AstResult;
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::EOI))]
struct _EOI;

impl IntoAst for ParserNodes<'_> {
    fn try_into_ast(mut self) -> AstResult {
        Ok(Ast::from_pest(&mut self)?)
    }
}

impl IntoAst for Source {
    fn try_into_ast(self) -> AstResult {
        let parsed = parse_source(&self)?;
        parsed.try_into_ast()
    }
}