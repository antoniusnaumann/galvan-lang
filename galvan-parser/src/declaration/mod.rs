mod r#fn;
mod ident;
mod tasks;
mod r#type;

pub use ident::*;
pub use r#fn::*;
pub use r#type::*;
pub use tasks::*;

#[derive(Debug)]
pub enum RootItem {
    Fn(FnDecl),
    Type(TypeDecl),
    Main(MainDecl),
    Test(TestDecl),
    CustomTask(TaskDecl),
}
