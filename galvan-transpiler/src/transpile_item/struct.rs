use std::collections::HashSet;

use galvan_ast::{
    DeclModifier, EnumTypeMember, Ident, StructTypeMember, TupleTypeMember, TypeDecl,
};

use crate::codegen::ref_storage_type;
use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::{ErrorCollector, Transpile};

static DERIVE: &str = "#[derive(Clone, Debug, PartialEq)]";
static REF_DERIVE: &str = "#[derive(Clone, Debug)]";

impl Transpile for TypeDecl {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self {
            TypeDecl::Tuple(def) => {
                let generic_params = generic_params(self.collect_generics());
                let visibility = def.visibility.transpile(ctx, errors);
                let ident = def.ident.transpile(ctx, errors);
                let field_visibility = visibility_prefix(&visibility);
                let members = def
                    .members
                    .iter()
                    .map(|member| {
                        format!("{field_visibility}{}", member.r#type.transpile(ctx, errors))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{DERIVE} {visibility} struct {ident}{generic_params}({members});")
            }
            TypeDecl::Struct(def) => {
                let generics = self.collect_generics();
                let generic_params = generic_params(generics.clone());
                let visibility = def.visibility.transpile(ctx, errors);
                let ident = def.ident.transpile(ctx, errors);
                let members = def
                    .members
                    .iter()
                    .map(|member| transpile_struct_member(member, &visibility, ctx, errors))
                    .collect::<Vec<_>>()
                    .join(",\n");
                let has_ref_fields = def
                    .members
                    .iter()
                    .any(|member| member.decl_modifier == Some(DeclModifier::Ref));
                if has_ref_fields {
                    let partial_eq = transpile_ref_partial_eq(def, &ident, generics, ctx, errors);
                    format!(
                        "{REF_DERIVE} {visibility} struct {ident}{generic_params} {{\n{members}\n}}\n\n{partial_eq}"
                    )
                } else {
                    format!(
                        "{DERIVE} {visibility} struct {ident}{generic_params} {{\n{members}\n}}"
                    )
                }
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

fn visibility_prefix(visibility: &str) -> String {
    if visibility.is_empty() {
        String::new()
    } else {
        format!("{visibility} ")
    }
}

fn transpile_ref_partial_eq(
    def: &galvan_ast::StructTypeDecl,
    ident: &str,
    generics: HashSet<Ident>,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    let impl_generics = generic_params_with_bound(generics.clone(), "PartialEq");
    let type_generics = generic_arguments(generics);
    let comparisons = def
        .members
        .iter()
        .map(|member| {
            let field = member.ident.transpile(ctx, errors);
            if member.decl_modifier == Some(DeclModifier::Ref) {
                format!("::galvan::std::__ref_value_eq(&self.{field}, &other.{field})")
            } else {
                format!("self.{field} == other.{field}")
            }
        })
        .collect::<Vec<_>>()
        .join(" && ");
    let comparisons = if comparisons.is_empty() {
        "true".to_string()
    } else {
        comparisons
    };

    format!(
        "impl{impl_generics} PartialEq for {ident}{type_generics} {{\n    fn eq(&self, other: &Self) -> bool {{\n        {comparisons}\n    }}\n}}"
    )
}

fn generic_params(generics: HashSet<Ident>) -> String {
    generic_params_with_bound(generics, "")
}

fn generic_params_with_bound(generics: HashSet<Ident>, additional_bound: &str) -> String {
    if generics.is_empty() {
        return String::new();
    }

    let mut generics = generics.into_iter().collect::<Vec<_>>();
    generics.sort_by(|left, right| left.as_str().cmp(right.as_str()));

    let params = generics
        .into_iter()
        .map(|g| {
            let additional_bound = if additional_bound.is_empty() {
                String::new()
            } else {
                format!(" + {additional_bound}")
            };
            format!(
                "{}: ToOwned<Owned = {}>{}",
                crate::capitalize_generic(g.as_str()),
                crate::capitalize_generic(g.as_str()),
                additional_bound
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{}>", params)
}

fn generic_arguments(generics: HashSet<Ident>) -> String {
    if generics.is_empty() {
        return String::new();
    }

    let mut generics = generics.into_iter().collect::<Vec<_>>();
    generics.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    format!(
        "<{}>",
        generics
            .into_iter()
            .map(|generic| crate::capitalize_generic(generic.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

impl_transpile!(TupleTypeMember, "{}", r#type);

impl Transpile for StructTypeMember {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        transpile_struct_member(self, "pub(crate)", ctx, errors)
    }
}

fn transpile_struct_member(
    member: &StructTypeMember,
    visibility: &str,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    let visibility = if visibility.is_empty() {
        String::new()
    } else {
        format!("{visibility} ")
    };

    match member.decl_modifier {
        Some(DeclModifier::Let) | Some(DeclModifier::Mut) | Some(DeclModifier::Move) => {
            errors.error_with_span(
                crate::TranspilerError::InvalidModifier {
                    modifier: "let/mut/move".to_string(),
                    context: "struct fields".to_string(),
                },
                Some(member.span.into()),
            );
            transpile!(
                ctx,
                errors,
                "{}{}: {}",
                visibility,
                member.ident,
                member.r#type
            )
        }
        Some(DeclModifier::Ref) => {
            let ty = member.r#type.transpile(ctx, errors);
            let storage_ty = ref_storage_type(&member.r#type, ty);
            transpile!(
                ctx,
                errors,
                "{}{}: {}",
                visibility,
                member.ident,
                storage_ty
            )
        }
        None => {
            transpile!(
                ctx,
                errors,
                "{}{}: {}",
                visibility,
                member.ident,
                member.r#type
            )
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
