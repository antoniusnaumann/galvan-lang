use crate::macros::{impl_transpile, impl_transpile_match, transpile};
use crate::{StructTypeMember, Transpile, TupleTypeMember, TypeDecl};
use galvan_ast::DeclModifier;
use galvan_resolver::LookupContext;

impl_transpile_match! { TypeDecl,
    Tuple(def) => ("{} struct {}({});", def.visibility, def.ident, def.members),
    Struct(def) => ("{} struct {} {{\n{}\n}}", def.visibility, def.ident, def.members),
    Alias(def) => ("{} type {} = {};", def.visibility, def.ident, def.r#type),
    Empty(def) => ("{} struct {};", def.visibility, def.ident),
}

impl_transpile!(TupleTypeMember, "{}", r#type);

impl Transpile for StructTypeMember {
    fn transpile(&self, lookup: &LookupContext) -> String {
        match self.decl_modifier {
            DeclModifier::Let => {
                todo!("Decide if let should be allowed on struct fields (and what it should mean")
            }
            DeclModifier::Mut => {
                todo!("Decide if mut should be allowed on struct fields (and what it should mean)")
            }
            DeclModifier::Ref => {
                transpile!(
                    lookup,
                    "pub(crate) {}: std::sync::Arc<std::sync::Mutex<{}>>",
                    self.ident,
                    self.r#type
                )
            }
            DeclModifier::Inherited => {
                transpile!(lookup, "pub(crate) {}: {}", self.ident, self.r#type)
            }
        }
    }
}
