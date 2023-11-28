use crate::macros::{impl_transpile, impl_transpile_match};
use crate::{StructTypeMember, TupleTypeMember, TypeDecl};

impl_transpile_match! { TypeDecl,
    Tuple(def) => ("{} struct {}({});", def.visibility, def.ident, def.members),
    Struct(def) => ("{} struct {} {{\n{}\n}}", def.visibility, def.ident, def.members),
    Alias(def) => ("{} type {} = {};", def.visibility, def.ident, def.r#type),
    Empty(def) => ("{} struct {};", def.visibility, def.ident),
}

impl_transpile!(TupleTypeMember, "{}", r#type);

// TODO: Probably generate getter/setter methods instead and never allow direct access to fields
impl_transpile!(StructTypeMember, "pub(crate) {}: {}", ident, r#type);
