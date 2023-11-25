use crate::{Transpile, MainDecl, transpile};

impl Transpile for MainDecl {
    fn transpile(self) -> String {
       transpile_main_decl(self)
    }
}

fn transpile_main_decl(decl: MainDecl) -> String {
    let MainDecl { body } = decl;
    transpile!("fn main() {{ {} }}", body)
}