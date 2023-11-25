use crate::{StructTypeMember, TupleTypeMember, TypeDecl};

use crate::{transpile, Transpile};

impl Transpile for TypeDecl {
    fn transpile(self) -> String {
        transpile_type_decl(self)
    }
}

fn transpile_type_decl(decl: TypeDecl) -> String {
    match decl {
        TypeDecl::Tuple(def) => transpile!("{} struct {}({});", def.visibility, def.ident, def.members),
        TypeDecl::Struct(def) => {
            if def.members.is_empty() {
                return transpile!("{} struct {} {{}}", def.visibility, def.ident);
            }
            transpile!("{} struct {} {{\n{}\n}}", def.visibility, def.ident, def.members)
        }
        TypeDecl::Alias(def) => transpile!("{} type {} = {};", def.visibility, def.ident, def.r#type),
        TypeDecl::Empty(def) => transpile!("{} struct {};", def.visibility, def.ident),
    }
}

impl Transpile for TupleTypeMember {
    fn transpile(self) -> String {
        transpile_tuple_type_member(self)
    }
}

fn transpile_tuple_type_member(member: TupleTypeMember) -> String {
    let TupleTypeMember { r#type } = member;
    transpile!("{}", r#type)
}

impl Transpile for StructTypeMember {
    fn transpile(self) -> String {
        transpile_struct_type_member(self)
    }
}

fn transpile_struct_type_member(member: StructTypeMember) -> String {
    let StructTypeMember {
        ident,
        r#type,
    } = member;
    transpile!("{}: {}", ident, r#type)
}
