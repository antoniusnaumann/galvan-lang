use galvan_parser::{TypeDecl, TypeDef};

use crate::{transpile, Transpile};

impl Transpile for TypeDecl {
    fn transpile(self) -> String {
        transpile_type_decl(self)
    }
}

fn transpile_type_decl(decl: TypeDecl) -> String {
    let TypeDecl {
        visibility,
        ident,
        def,
    } = decl;
    match def {
        TypeDef::TupleType(def) => transpile!("{} struct {}({})", visibility, ident, def.members),
        TypeDef::StructType(def) => {
            transpile!("{} struct {} {{ {} }}", visibility, ident, def.members)
        }
        TypeDef::AliasType(def) => transpile!("{} type {} = {}", visibility, ident, def.r#type),
    }
}
