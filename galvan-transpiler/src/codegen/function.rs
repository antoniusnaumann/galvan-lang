use std::collections::HashSet;

use galvan_ast::{CmdSignature, DeclModifier, FnSignature, Ident, Param, TypeElement};
use galvan_hir::builtins::CheckBuiltins;
use galvan_hir::hir::{HirFunction, HirMain, HirMainKind, HirTest};
use itertools::Itertools;

use crate::codegen::ref_storage_type;
use crate::context::Context;
use crate::sanitize::{mangle_function_name, sanitize_name};
use crate::transpile_item::ident::{TranspileType, TypeOwnership};
use crate::ErrorCollector;
use crate::Transpile;

/// Transpiles a function. Generic parameters that are already declared by a
/// surrounding `impl` block are skipped instead of being re-declared on the
/// function.
pub(crate) fn transpile_function(
    function: &HirFunction,
    ctx: &Context,
    errors: &mut ErrorCollector,
    skip_generics: &HashSet<Ident>,
) -> String {
    let signature = transpile_signature(&function.signature, ctx, errors, skip_generics);
    let block = function.body.transpile(ctx, errors);

    if !function.signature.return_type.is_void() {
        format!("{signature} {block}")
    } else {
        format!("{signature} {{ {block}; }}")
    }
}

pub(crate) fn transpile_signature(
    signature: &FnSignature,
    ctx: &Context,
    errors: &mut ErrorCollector,
    skip_generics: &HashSet<Ident>,
) -> String {
    let visibility = signature.visibility.transpile(ctx, errors);
    let identifier =
        mangle_function_name(signature.identifier.as_str(), signature.overload_labels());

    let generics = signature.collect_generics();
    let mut generics = generics
        .difference(skip_generics)
        .map(|generic| crate::capitalize_generic(generic.as_str()))
        .collect::<Vec<_>>();
    generics.sort();
    let generic_params = if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    };

    let parameters = format!(
        "({})",
        signature
            .parameters
            .params
            .iter()
            .map(|param| param.transpile(ctx, errors))
            .join(", ")
    );

    let return_type = match &signature.return_type {
        TypeElement::Infer(_) | TypeElement::Void(_) => String::new(),
        ty => format!(" -> {}", ty.transpile(ctx, errors)),
    };

    let where_clause = if let Some(where_clause) = &signature.where_clause {
        let constraints = where_clause
            .bounds
            .iter()
            .flat_map(|bound| {
                let trait_bounds = bound
                    .bounds
                    .iter()
                    .map(|bound| bound.as_str())
                    .collect::<Vec<_>>()
                    .join(" + ");
                bound.type_params.iter().map(move |param| {
                    format!(
                        "{}: {}",
                        crate::capitalize_generic(param.as_str()),
                        trait_bounds
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!(" where {constraints}")
    } else {
        String::new()
    };

    format!("{visibility} fn {identifier}{generic_params}{parameters}{return_type}{where_clause}")
}

impl Transpile for Param {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let is_self = self.identifier.is_self();
        let is_copy = ctx.mapping.is_copy(&self.param_type);

        match self.decl_modifier {
            Some(DeclModifier::Let) | None => {
                if is_self {
                    if is_copy {
                        "self".into()
                    } else {
                        "&self".into()
                    }
                } else {
                    let ownership = if is_copy {
                        TypeOwnership::Owned
                    } else {
                        TypeOwnership::Borrowed
                    };
                    transpile_param(self, ctx, ownership, errors)
                }
            }
            Some(DeclModifier::Move) => {
                if is_self {
                    "self".into()
                } else {
                    transpile_param(self, ctx, TypeOwnership::Owned, errors)
                }
            }
            Some(DeclModifier::Mut) => {
                if is_self {
                    "&mut self".into()
                } else {
                    transpile_param(self, ctx, TypeOwnership::MutBorrowed, errors)
                }
            }
            Some(DeclModifier::Ref) => {
                if is_self {
                    return "__self: std::sync::Arc<std::sync::Mutex<Self>>".into();
                }

                let ty = self.param_type.transpile(ctx, errors);
                format!(
                    "{}: {}",
                    sanitize_name(self.identifier.as_str()),
                    ref_storage_type(&self.param_type, ty),
                )
            }
        }
    }
}

fn transpile_param(
    param: &Param,
    ctx: &Context,
    ownership: TypeOwnership,
    errors: &mut ErrorCollector,
) -> String {
    let mut prefix = "";
    let ty = match &param.param_type {
        TypeElement::Plain(plain) => plain.ident.transpile_type(ctx, ownership, errors),
        other => {
            match ownership {
                TypeOwnership::Borrowed => prefix = "&",
                TypeOwnership::MutBorrowed => prefix = "&mut ",
                TypeOwnership::Owned | TypeOwnership::MutOwned => {}
            }
            other.transpile(ctx, errors)
        }
    };

    format!(
        "{}: {}{}",
        sanitize_name(param.identifier.as_str()),
        prefix,
        ty
    )
}

pub(crate) fn transpile_test(
    name: &str,
    test: &HirTest,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    let body = test.body.transpile(ctx, errors);
    format!("#[test]\nfn {name}() {{\n{body};\n}}")
}

pub(crate) fn transpile_main(main: &HirMain, ctx: &Context, errors: &mut ErrorCollector) -> String {
    let body = main.body.transpile(ctx, errors);
    match &main.kind {
        HirMainKind::Function { argument: None } => {
            format!("pub(crate) fn __main__() {body}")
        }
        HirMainKind::Function {
            argument: Some(argument),
        } => {
            let argument = sanitize_name(argument.as_str());
            format!(
                "pub(crate) fn __main__() {{ let {argument}: ::std::vec::Vec<String> = ::std::env::args().collect(); {body}; }}"
            )
        }
        HirMainKind::Command { signature } => {
            let parameters = signature
                .parameters
                .params
                .iter()
                .map(|param| {
                    format!(
                        "{}: {}",
                        sanitize_name(param.identifier.as_str()),
                        param.param_type.transpile(ctx, errors)
                    )
                })
                .join(", ");
            format!(
                "pub(crate) fn __main__() {{ unreachable!(\"CLI entry point dispatches through __cli_main\") }}\nfn __main_command({parameters}) {body}"
            )
        }
    }
}

impl Transpile for CmdSignature {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        // CLI commands transpile to a regular function that is always
        // private and returns ()
        let identifier = sanitize_name(self.identifier.as_str());

        let parameters = self
            .parameters
            .params
            .iter()
            .map(|param| {
                format!(
                    "{}: {}",
                    sanitize_name(param.identifier.as_str()),
                    param.param_type.transpile(ctx, errors)
                )
            })
            .join(", ");

        format!("fn {identifier}({parameters})")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use galvan_ast::{
        BasicTypeItem, EmptyTypeDecl, FnSignature, Ident, ParamList, ParametricTypeItem,
        SegmentedAsts, Span, ToplevelItem, TypeDecl, TypeElement, TypeIdent, Visibility,
    };
    use galvan_files::Source;
    use galvan_hir::mapping::{Mapping, RustType};

    use super::*;

    fn empty_type(name: &str) -> ToplevelItem<TypeDecl> {
        ToplevelItem {
            item: TypeDecl::Empty(EmptyTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new(name),
                generic_params: Vec::new(),
                span: Span::default(),
            }),
            source: Source::Builtin,
        }
    }

    fn context_with_json_mapping() -> Context<'static> {
        let mut mapping = Mapping::default();
        mapping.types.insert(
            TypeIdent::new("Json"),
            RustType::new("::axum::Json", "::axum::Json", "::axum::Json", false),
        );

        let segmented = Box::leak(Box::new(SegmentedAsts {
            uses: Vec::new(),
            types: vec![
                ToplevelItem {
                    item: TypeDecl::Empty(EmptyTypeDecl {
                        visibility: Visibility::public(),
                        ident: TypeIdent::new("Json"),
                        generic_params: vec![Ident::new("T")],
                        span: Span::default(),
                    }),
                    source: Source::Builtin,
                },
                empty_type("HealthResponse"),
                empty_type("TicketResponse"),
            ],
            functions: Vec::new(),
            tests: Vec::new(),
            main: None,
            cmds: Vec::new(),
        }));

        Context::new(mapping).with(segmented)
    }

    fn signature_with_return(return_type: TypeElement) -> FnSignature {
        FnSignature {
            visibility: Visibility::public(),
            identifier: Ident::new("health"),
            parameters: ParamList {
                params: Vec::new(),
                span: Span::default(),
            },
            return_type,
            where_clause: None,
            span: Span::default(),
        }
    }

    #[test]
    fn generic_external_return_types_render_qualified_rust_path() {
        let ctx = context_with_json_mapping();
        let mut errors = ErrorCollector::new();
        let signature = signature_with_return(TypeElement::Parametric(ParametricTypeItem {
            base_type: TypeIdent::new("Json"),
            type_args: vec![TypeElement::Plain(BasicTypeItem {
                ident: TypeIdent::new("HealthResponse"),
                span: Span::default(),
            })],
            span: Span::default(),
        }));

        let rendered = transpile_signature(&signature, &ctx, &mut errors, &HashSet::new());

        assert_eq!(rendered, "pub fn health() -> ::axum::Json<HealthResponse>");
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn generic_external_array_return_types_render_qualified_rust_path() {
        let ctx = context_with_json_mapping();
        let mut errors = ErrorCollector::new();
        let signature = signature_with_return(TypeElement::Parametric(ParametricTypeItem {
            base_type: TypeIdent::new("Json"),
            type_args: vec![TypeElement::Array(Box::new(galvan_ast::ArrayTypeItem {
                elements: TypeElement::Plain(BasicTypeItem {
                    ident: TypeIdent::new("TicketResponse"),
                    span: Span::default(),
                }),
                span: Span::default(),
            }))],
            span: Span::default(),
        }));

        let rendered = transpile_signature(&signature, &ctx, &mut errors, &HashSet::new());

        assert_eq!(
            rendered,
            "pub fn health() -> ::axum::Json<::std::vec::Vec<TicketResponse>>"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }
}
