use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_match, transpile};
use crate::{StructTypeMember, Transpile, TupleTypeMember, TypeDecl};
use galvan_ast::{DeclModifier, EnumAccess, EnumTypeMember, EnumVariantField};
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
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut crate::ErrorCollector) -> String {
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
                    errors,
                    "pub(crate) {}: std::sync::Arc<std::sync::Mutex<{}>>",
                    self.ident,
                    self.r#type
                )
            }
            None => {
                transpile!(ctx, scope, errors, "pub(crate) {}: {}", self.ident, self.r#type)
            }
        }
    }
}

impl Transpile for EnumTypeMember {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut crate::ErrorCollector) -> String {
        if self.fields.is_empty() {
            // Simple variant: Transparent
            format!("{}", self.ident)
        } else if self.fields.iter().all(|f| f.name.is_none()) {
            // All anonymous fields: Gray(u8) 
            let types: Vec<_> = self.fields.iter().map(|f| f.r#type.transpile(ctx, scope, errors)).collect();
            format!("{}({})", self.ident, types.join(", "))
        } else {
            // Named fields: Rgb { r: u8, g: u8, b: u8 }
            let field_defs: Vec<_> = self.fields.iter().map(|f| {
                if let Some(ref name) = f.name {
                    format!("{}: {}", name.as_str(), f.r#type.transpile(ctx, scope, errors))
                } else {
                    // Mix of named and unnamed should not be allowed
                    errors.error(crate::TranspilerError::InvalidSyntax { 
                        message: "Cannot mix named and unnamed fields in enum variant".to_string() 
                    });
                    f.r#type.transpile(ctx, scope, errors)
                }
            }).collect();
            format!("{} {{ {} }}", self.ident, field_defs.join(", "))
        }
    }
}

impl Transpile for EnumAccess {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut crate::ErrorCollector) -> String {
        // TODO: Fully qualified path
        transpile!(ctx, scope, errors, "{}::{}", self.target, self.case.as_str())
    }
}
