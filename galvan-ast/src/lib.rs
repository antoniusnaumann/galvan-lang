#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate pest_ast;

use derive_more::From;
use from_pest::{ConversionError, FromPest, Void};

use galvan_pest::*;
pub use galvan_pest::{Source};

mod r#fn;
mod ident;
mod tasks;
mod r#type;
mod modifier;
mod literal;

pub use ident::*;
pub use r#fn::*;
pub use r#type::*;
pub use tasks::*;
pub use modifier::*;
pub use literal::*;

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

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::toplevel))]
pub enum RootItem {
    // Fn(FnDecl),
    Type(TypeDecl),
    Main(MainDecl),
    Test(TestDecl),
    CustomTask(TaskDecl),
}

impl From<RootItem> for Ast {
    fn from(item: RootItem) -> Self {
        Ast::new(vec![item])
    }
}

fn string(span: BorrowedSpan<'_>) -> String {
    span.as_str().to_owned()
}

pub type AstError = ConversionError<Void>;
pub type AstResult = Result<Ast, AstError>;

pub trait IntoAst {
    fn try_into_ast(self) -> AstResult;
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::EOI))]
struct _EOI;

impl IntoAst for ParserNodes<'_> {
    fn try_into_ast(mut self) -> AstResult {
        Ast::from_pest(&mut self)
    }
}