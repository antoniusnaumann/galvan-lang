use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_match, transpile};
use crate::{StructTypeMember, Transpile, TupleTypeMember, TypeDecl};
use galvan_ast::{DeclModifier, EnumAccess, EnumTypeMember};
use galvan_resolver::Scope;

static DERIVE: &str = "#[derive(Clone, Debug, PartialEq)]";

impl_transpile_match! { TypeDecl,
    Tuple(def) => ("{DERIVE} {} struct {}({});", def.visibility, def.ident, def.members),
    Struct(def) => ("{DERIVE} {} struct {} {{\n{}\n}}", def.visibility, def.ident, def.members),
    Enum(def) => ("{DERIVE} {} enum {} {{\n{}\n}}", def.visibility, def.ident, def.members),
    Alias(def) => ("{} type {} = {};", def.visibility, def.ident, def.r#type),
    Empty(def) => ("{DERIVE} {} struct {};", def.visibility, def.ident),
}

impl_transpile!(TupleTypeMember, "{}", r#type);

impl Transpile for StructTypeMember {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        match self.decl_modifier {
            Some(DeclModifier::Let) => {
                todo!("Decide if let should be allowed on struct fields (and what it should mean")
            }
            Some(DeclModifier::Mut) => {
                todo!("Decide if mut should be allowed on struct fields (and what it should mean)")
            }
            Some(DeclModifier::Ref) => {
                transpile!(
                    ctx,
                    scope,
                    "pub(crate) {}: std::sync::Arc<std::sync::Mutex<{}>>",
                    self.ident,
                    self.r#type
                )
            }
            None => {
                transpile!(ctx, scope, "pub(crate) {}: {}", self.ident, self.r#type)
            }
        }
    }
}

impl Transpile for EnumTypeMember {
    fn transpile(&self, _ctx: &Context, _scope: &mut Scope) -> String {
        format!("{}", self.ident)
    }
}

impl Transpile for EnumAccess {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        // TODO: Fully qualified path
        transpile!(ctx, scope, "{}::{}", self.target, self.case.as_str())
    }
}
