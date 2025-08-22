use std::collections::HashSet;
use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::{StructTypeMember, Transpile, TupleTypeMember, TypeDecl};
use galvan_ast::{DeclModifier, EnumAccess, EnumTypeMember};
use galvan_resolver::Scope;

static DERIVE: &str = "#[derive(Clone, Debug, PartialEq)]";

impl Transpile for TypeDecl {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut crate::ErrorCollector) -> String {
        match self {
            TypeDecl::Tuple(def) => {
                // Collect generic parameters from tuple members
                let mut generics = HashSet::new();
                for member in &def.members {
                    member.r#type.collect_generics_recursive(&mut generics);
                }
                
                let generic_params = if generics.is_empty() {
                    String::new()
                } else {
                    // Add ToOwned trait bound to all generic parameters for Galvan's ownership semantics
                    let params = generics.iter().map(|g| format!("{}: ToOwned<Owned = {}>", g.as_str(), g.as_str())).collect::<Vec<_>>().join(", ");
                    format!("<{}>", params)
                };
                
                let visibility = def.visibility.transpile(ctx, scope, errors);
                let ident = def.ident.transpile(ctx, scope, errors);
                let members = def.members.transpile(ctx, scope, errors);
                format!("{DERIVE} {visibility} struct {ident}{generic_params}({members});")
            },
            TypeDecl::Struct(def) => {
                // Collect generic parameters from struct members
                let mut generics = HashSet::new();
                for member in &def.members {
                    member.r#type.collect_generics_recursive(&mut generics);
                }
                
                let generic_params = if generics.is_empty() {
                    String::new()
                } else {
                    // Add ToOwned trait bound to all generic parameters for Galvan's ownership semantics
                    let params = generics.iter().map(|g| format!("{}: ToOwned<Owned = {}>", g.as_str(), g.as_str())).collect::<Vec<_>>().join(", ");
                    format!("<{}>", params)
                };
                
                let visibility = def.visibility.transpile(ctx, scope, errors);
                let ident = def.ident.transpile(ctx, scope, errors);
                let members = def.members.transpile(ctx, scope, errors);
                format!("{DERIVE} {visibility} struct {ident}{generic_params} {{\n{members}\n}}")
            },
            TypeDecl::Enum(def) => {
                let visibility = def.visibility.transpile(ctx, scope, errors);
                let ident = def.ident.transpile(ctx, scope, errors);
                let members = def.members.transpile(ctx, scope, errors);
                format!("{DERIVE} {visibility} enum {ident} {{\n{members}\n}}")
            },
            TypeDecl::Alias(def) => {
                let visibility = def.visibility.transpile(ctx, scope, errors);
                let ident = def.ident.transpile(ctx, scope, errors);
                let r#type = def.r#type.transpile(ctx, scope, errors);
                format!("{visibility} type {ident} = {type};")
            },
            TypeDecl::Empty(def) => {
                let visibility = def.visibility.transpile(ctx, scope, errors);
                let ident = def.ident.transpile(ctx, scope, errors);
                format!("{DERIVE} {visibility} struct {ident};")
            },
        }
    }
}

impl_transpile!(TupleTypeMember, "{}", r#type);

impl Transpile for StructTypeMember {
    fn transpile(
        &self,
        ctx: &Context,
        scope: &mut Scope,
        errors: &mut crate::ErrorCollector,
    ) -> String {
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
                transpile!(
                    ctx,
                    scope,
                    errors,
                    "pub(crate) {}: {}",
                    self.ident,
                    self.r#type
                )
            }
        }
    }
}

impl Transpile for EnumTypeMember {
    fn transpile(
        &self,
        ctx: &Context,
        scope: &mut Scope,
        errors: &mut crate::ErrorCollector,
    ) -> String {
        if self.fields.is_empty() {
            // Simple variant: Transparent
            format!("{}", self.ident)
        } else if self.fields.iter().all(|f| f.name.is_none()) {
            // All anonymous fields: Gray(u8)
            let types: Vec<_> = self
                .fields
                .iter()
                .map(|f| f.r#type.transpile(ctx, scope, errors))
                .collect();
            format!("{}({})", self.ident, types.join(", "))
        } else {
            // Named fields: Rgb { r: u8, g: u8, b: u8 }
            let field_defs: Vec<_> = self
                .fields
                .iter()
                .map(|f| {
                    if let Some(ref name) = f.name {
                        format!(
                            "{}: {}",
                            name.as_str(),
                            f.r#type.transpile(ctx, scope, errors)
                        )
                    } else {
                        // Mix of named and unnamed should not be allowed
                        errors.error(crate::TranspilerError::InvalidSyntax {
                            message: "Cannot mix named and unnamed fields in enum variant"
                                .to_string(),
                        });
                        f.r#type.transpile(ctx, scope, errors)
                    }
                })
                .collect();
            format!("{} {{ {} }}", self.ident, field_defs.join(", "))
        }
    }
}

impl Transpile for EnumAccess {
    fn transpile(
        &self,
        ctx: &Context,
        scope: &mut Scope,
        errors: &mut crate::ErrorCollector,
    ) -> String {
        // TODO: Fully qualified path
        transpile!(
            ctx,
            scope,
            errors,
            "{}::{}",
            self.target,
            self.case.as_str()
        )
    }
}