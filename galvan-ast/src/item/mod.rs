mod assignment;
mod closure;
mod r#fn;
mod function_call;
mod ident;
mod literal;
mod modifier;
mod operator;
mod statement;
mod tasks;
mod toplevel;
mod r#type;
mod type_item;

pub use assignment::*;
pub use closure::*;
pub use function_call::*;
pub use ident::*;
pub use literal::*;
pub use modifier::*;
pub use operator::*;
pub use r#fn::*;
pub use r#type::*;
pub use statement::*;
pub use tasks::*;
pub use toplevel::*;
pub use type_item::*;

fn string(span: galvan_pest::BorrowedSpan<'_>) -> String {
    span.as_str().to_owned()
}
