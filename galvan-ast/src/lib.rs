#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate pest_ast;

use derive_more::From;
use from_pest::{ConversionError, FromPest, Void};

use galvan_pest::*;

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

#[derive(Debug, From, FromPest)]
#[pest_ast(rule(Rule::source))]
pub struct Ast(Vec<RootItem>, _EOI);

impl Ast {
    pub fn root_items(&self) -> &[RootItem] {
        &self.0
    }

    pub fn into_root_items(self) -> Vec<RootItem> {
        self.0
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::EOI))]
struct _EOI;

#[derive(Debug, From, FromPest)]
#[pest_ast(rule(Rule::toplevel))]
pub enum RootItem {
    // Fn(FnDecl),
    Type(TypeDecl),
    Main(MainDecl),
    Test(TestDecl),
    CustomTask(TaskDecl),
}

fn string(span: BorrowedSpan<'_>) -> String {
    span.as_str().to_owned()
}

pub type AstError = ConversionError<Void>;
pub type AstResult = Result<Ast, AstError>;

pub trait IntoAst {
    fn try_into_ast(self) -> AstResult;
}

impl IntoAst for ParserNodes<'_> {
    fn try_into_ast(mut self) -> AstResult {
        Ast::from_pest(&mut self)
    }
}