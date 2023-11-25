mod r#fn;
mod ident;
mod tasks;
mod r#type;
mod modifier;
mod literal;
mod toplevel;

pub use ident::*;
pub use r#fn::*;
pub use r#type::*;
pub use tasks::*;
pub use modifier::*;
pub use literal::*;
pub use toplevel::*;

fn string(span: galvan_pest::BorrowedSpan<'_>) -> String {
    span.as_str().to_owned()
}