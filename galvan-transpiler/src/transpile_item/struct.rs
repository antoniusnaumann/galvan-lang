use crate::codegen::ref_storage_type;
use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::{ErrorCollector, Transpile};
use galvan_ast::{
    DeclModifier, EnumTypeMember, Ident, StructTypeMember, TupleTypeMember, TypeDecl,
};
use std::collections::HashSet;

static DERIVE: &str = "#[derive(Clone, Debug, PartialEq)]";

impl Transpile for TypeDecl {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self {
            TypeDecl::Tuple(def) => {
                let generic_params = generic_params(self.collect_generics());
                let visibility = def.visibility.transpile(ctx, errors);
                let ident = def.ident.transpile(ctx, errors);
                let members = def.members.transpile(ctx, errors);
                format!("{DERIVE} {visibility} struct {ident}{generic_params}({members});")
            }
            TypeDecl::Struct(def) => {
                let generic_params = generic_params(self.collect_generics());
                let visibility = def.visibility.transpile(ctx, errors);
                let ident = def.ident.transpile(ctx, errors);
                let members = def.members.transpile(ctx, errors);
                format!("{DERIVE} {visibility} struct {ident}{generic_params} {{\n{members}\n}}")
            }
            TypeDecl::Enum(def) => {
                let generic_params = generic_params(self.collect_generics());
                let visibility = def.visibility.transpile(ctx, errors);
                let ident = def.ident.transpile(ctx, errors);
                let members = def.members.transpile(ctx, errors);
                format!("{DERIVE} {visibility} enum {ident}{generic_params} {{\n{members}\n}}")
            }
            TypeDecl::Alias(def) => {
                let generic_params = generic_params(self.collect_generics());
                let visibility = def.visibility.transpile(ctx, errors);
                let ident = def.ident.transpile(ctx, errors);
                let r#type = def.r#type.transpile(ctx, errors);
                format!("{visibility} type {ident}{generic_params} = {type};")
            }
            TypeDecl::Empty(def) => {
                let generic_params = generic_params(self.collect_generics());
                let visibility = def.visibility.transpile(ctx, errors);
                let ident = def.ident.transpile(ctx, errors);
                format!("{DERIVE} {visibility} struct {ident}{generic_params};")
            }
        }
    }
}

fn generic_params(generics: HashSet<Ident>) -> String {
    if generics.is_empty() {
        return String::new();
    }

    let mut generics = generics.into_iter().collect::<Vec<_>>();
    generics.sort_by(|left, right| left.as_str().cmp(right.as_str()));

    let params = generics
        .into_iter()
        .map(|g| {
            format!(
                "{}: ToOwned<Owned = {}>",
                crate::capitalize_generic(g.as_str()),
                crate::capitalize_generic(g.as_str())
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{}>", params)
}

impl_transpile!(TupleTypeMember, "{}", r#type);

impl Transpile for StructTypeMember {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self.decl_modifier {
            Some(DeclModifier::Let) | Some(DeclModifier::Mut) | Some(DeclModifier::Move) => {
                errors.error_with_span(
                    crate::TranspilerError::InvalidModifier {
                        modifier: "let/mut/move".to_string(),
                        context: "struct fields".to_string(),
                    },
                    Some(self.span.into()),
                );
                transpile!(ctx, errors, "pub(crate) {}: {}", self.ident, self.r#type)
            }
            Some(DeclModifier::Ref) => {
                let ty = self.r#type.transpile(ctx, errors);
                let storage_ty = ref_storage_type(&self.r#type, ty);
                transpile!(ctx, errors, "pub(crate) {}: {}", self.ident, storage_ty)
            }
            None => {
                transpile!(ctx, errors, "pub(crate) {}: {}", self.ident, self.r#type)
            }
        }
    }
}

impl Transpile for EnumTypeMember {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        if self.fields.is_empty() {
            // Simple variant: Transparent
            format!("{}", self.ident)
        } else if self.fields.iter().all(|f| f.name.is_none()) {
            // All anonymous fields: Gray(u8)
            let types: Vec<_> = self
                .fields
                .iter()
                .map(|f| f.r#type.transpile(ctx, errors))
                .collect();
            format!("{}({})", self.ident, types.join(", "))
        } else {
            // Named fields: Rgb { r: u8, g: u8, b: u8 }
            let field_defs: Vec<_> = self
                .fields
                .iter()
                .map(|f| {
                    if let Some(ref name) = f.name {
                        format!("{}: {}", name.as_str(), f.r#type.transpile(ctx, errors))
                    } else {
                        // Mix of named and unnamed should not be allowed
                        errors.error_with_span(
                            crate::TranspilerError::InvalidSyntax {
                                message: "Cannot mix named and unnamed fields in enum variant"
                                    .to_string(),
                            },
                            Some(f.span.into()),
                        );
                        f.r#type.transpile(ctx, errors)
                    }
                })
                .collect();
            format!("{} {{ {} }}", self.ident, field_defs.join(", "))
        }
    }
}
