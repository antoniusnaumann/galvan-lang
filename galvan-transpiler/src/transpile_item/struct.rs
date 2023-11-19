use galvan_parser::{StructTypeMember, TupleTypeMember, TypeDecl, TypeDecl};

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
        TypeDecl::TupleType(def) => transpile!("{} struct {}({});", visibility, ident, def.members),
        TypeDecl::StructType(def) => {
            transpile!("{} struct {} {{ {} }}", visibility, ident, def.members)
        }
        TypeDecl::AliasType(def) => transpile!("{} type {} = {};", visibility, ident, def.r#type),
    }
}

impl Transpile for TupleTypeMember {
    fn transpile(self) -> String {
        transpile_tuple_type_member(self)
    }
}

fn transpile_tuple_type_member(member: TupleTypeMember) -> String {
    let TupleTypeMember { visibility, r#type } = member;
    transpile!("{} {}", visibility, r#type)
}

impl Transpile for StructTypeMember {
    fn transpile(self) -> String {
        transpile_struct_type_member(self)
    }
}

fn transpile_struct_type_member(member: StructTypeMember) -> String {
    let StructTypeMember {
        visibility,
        ident,
        r#type,
    } = member;
    transpile!("{} {}: {}", visibility, ident, r#type)
}
