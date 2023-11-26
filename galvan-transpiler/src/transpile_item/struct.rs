use crate::{StructTypeMember, TupleTypeMember, TypeDecl};
use crate::macros::{impl_transpile, impl_transpile_match};


impl_transpile_match! { TypeDecl,
    Tuple(def) => ("{} struct {}({});", def.visibility, def.ident, def.members),
    Struct(def) => ("{} struct {} {{\n{}\n}}", def.visibility, def.ident, def.members),
    Alias(def) => ("{} type {} = {};", def.visibility, def.ident, def.r#type),
    Empty(def) => ("{} struct {};", def.visibility, def.ident),
}

impl_transpile!(TupleTypeMember, "{}", r#type);
impl_transpile!(StructTypeMember, "{}: {}", ident, r#type);
