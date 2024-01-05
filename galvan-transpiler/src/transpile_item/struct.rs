use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_match, transpile};
use crate::{StructTypeMember, Transpile, TupleTypeMember, TypeDecl};
use galvan_ast::DeclModifier;

static DERIVE: &str = "#[derive(Clone, Debug, PartialEq)]";

impl_transpile_match! { TypeDecl,
    Tuple(def) => ("{DERIVE} {} struct {}({});", def.visibility, def.ident, def.members),
    Struct(def) => ("{DERIVE} {} struct {} {{\n{}\n}}", def.visibility, def.ident, def.members),
    Alias(def) => ("{} type {} = {};", def.visibility, def.ident, def.r#type),
    Empty(def) => ("{DERIVE} {} struct {};", def.visibility, def.ident),
}

impl_transpile!(TupleTypeMember, "{}", r#type);

impl Transpile for StructTypeMember {
    fn transpile(&self, ctx: &Context) -> String {
        match self.decl_modifier {
            DeclModifier::Let => {
                todo!("Decide if let should be allowed on struct fields (and what it should mean")
            }
            DeclModifier::Mut => {
                todo!("Decide if mut should be allowed on struct fields (and what it should mean)")
            }
            DeclModifier::Ref => {
                transpile!(
                    ctx,
                    "pub(crate) {}: std::sync::Arc<std::sync::Mutex<{}>>",
                    self.ident,
                    self.r#type
                )
            }
            DeclModifier::Inherited => {
                transpile!(ctx, "pub(crate) {}: {}", self.ident, self.r#type)
            }
        }
    }
}
