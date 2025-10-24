use std::borrow::Cow;
use std::collections::HashMap;
use std::iter;

use convert_case::{Case, Casing};
use derive_more::{Deref, Display, From};
use itertools::Itertools;
use thiserror::Error;

use galvan_ast::*;
use galvan_files::{FileError, Source};
use galvan_into_ast::{AstError, SegmentAst, SourceIntoAst};
use galvan_resolver::{LookupError, Scope};

use builtins::builtin_fns;

static SUPPRESS_WARNINGS: &str = "#![allow(warnings, unused)]";

// TODO: Maybe use something like https://crates.io/crates/ruast to generate the Rust code in a more reliable way

/// Helper function to capitalize first letter of generic type parameters for Rust convention
fn capitalize_generic(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Name of the generated rust module that exports all public items from all galvan files in this crate
#[macro_export]
macro_rules! galvan_module {
    () => {
        "galvan_module"
    };
    ($ext:literal) => {
        concat!("galvan_module.", $ext)
    };
}

/// Extract doc comment from source code by looking backwards from a given span
fn extract_doc_comment(source_content: &str, span: &galvan_ast::Span) -> Option<String> {
    let lines: Vec<&str> = source_content.lines().collect();

    // Find the line before the span
    if span.start.row == 0 {
        return None;
    }

    let mut doc_lines = Vec::new();
    let mut current_row = span.start.row;

    // Look backwards from the span line to find doc comments
    while current_row > 0 {
        current_row -= 1;

        if let Some(line) = lines.get(current_row) {
            let trimmed = line.trim();

            if trimmed.starts_with("///") {
                // Extract the doc comment content (remove /// and trim)
                let comment_content = trimmed.strip_prefix("///").unwrap_or("").trim();
                doc_lines.insert(0, comment_content.to_string());
            } else if trimmed.is_empty() || trimmed.starts_with("//") {
                // Empty lines and regular comments are allowed between doc comments and the target
                continue;
            } else {
                // Non-comment line, stop looking
                break;
            }
        } else {
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join(" "))
    }
}

/// Extract doc comment for a parameter from source code
fn extract_param_doc_comment(source_content: &str, param: &Param) -> Option<String> {
    extract_doc_comment(source_content, &param.span)
}

/// Generate CLI structure with subcommands
fn generate_cli_structure(
    commands: &[ToplevelItem<CmdDecl>],
    ctx: &Context,
    scope: &mut Scope,
    errors: &mut ErrorCollector,
) -> (String, String) {
    let mut command_functions = Vec::new();
    let mut subcommand_variants = Vec::new();
    let mut subcommand_args = Vec::new();
    let mut match_arms = Vec::new();

    for cmd in commands {
        let cmd_name = cmd.item.signature.identifier.as_str();
        let cmd_name_pascal = cmd_name.to_case(Case::Pascal);

        // Generate the command function
        let function_code = cmd.transpile(ctx, scope, errors);
        command_functions.push(function_code);

        // Generate args struct for this command
        let mut args_fields = Vec::new();
        let mut function_params = Vec::new();

        for param in &cmd.item.signature.parameters.params {
            let field_name = param.identifier.as_str();
            let param_type = param.param_type.transpile(ctx, scope, errors);

            // Extract doc comment for this parameter
            let help_text = extract_param_doc_comment(cmd.source.content(), param);

            // Generate clap attribute based on short_name and help text
            let clap_attr = match (&param.short_name, &help_text) {
                (Some(short_name), Some(help)) => {
                    format!(
                        "#[arg(short = '{}', long = \"{}\", help = \"{}\")]",
                        short_name.as_str(),
                        field_name,
                        help
                    )
                }
                (Some(short_name), None) => {
                    format!(
                        "#[arg(short = '{}', long = \"{}\")]",
                        short_name.as_str(),
                        field_name
                    )
                }
                (None, Some(help)) => {
                    format!("#[arg(long = \"{}\", help = \"{}\")]", field_name, help)
                }
                (None, None) => {
                    format!("#[arg(long = \"{}\")]", field_name)
                }
            };

            args_fields.push(format!(
                "    {}\n    pub {}: {}",
                clap_attr, field_name, param_type
            ));
            function_params.push(format!("args.{}", field_name));
        }

        let args_struct = if args_fields.is_empty() {
            format!(
                "#[derive(clap::Args, Debug)]\nstruct {}Args {{}}",
                cmd_name_pascal
            )
        } else {
            format!(
                "#[derive(clap::Args, Debug)]\nstruct {}Args {{\n{}\n}}",
                cmd_name_pascal,
                args_fields.join(",\n")
            )
        };

        subcommand_args.push(args_struct);

        // Extract doc comment for the command itself
        let cmd_help = extract_doc_comment(cmd.source.content(), &cmd.item.span);

        // Generate subcommand enum variant with help text
        let variant = if let Some(help) = cmd_help {
            format!(
                "    /// {}\n    {} ({}Args)",
                help, cmd_name_pascal, cmd_name_pascal
            )
        } else {
            format!("    {} ({}Args)", cmd_name_pascal, cmd_name_pascal)
        };
        subcommand_variants.push(variant);

        // Generate match arm
        let function_call = if function_params.is_empty() {
            format!("{}()", cmd_name)
        } else {
            format!("{}({})", cmd_name, function_params.join(", "))
        };

        match_arms.push(format!(
            "        Some(Commands::{}(args)) => {},",
            cmd_name_pascal, function_call
        ));
    }

    let cli_code = format!(
        r#"
use clap::{{Parser, Subcommand, Args}};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {{
    #[command(subcommand)]
    command: Option<Commands>,
}}

#[derive(Subcommand)]
enum Commands {{
{}
}}

{}

pub(crate) fn __cli_main() {{
    let cli = Cli::parse();
    match cli.command {{
{}
        None => __main__(),
    }}
}}
"#,
        subcommand_variants.join(",\n"),
        subcommand_args.join("\n\n"),
        match_arms.join("\n")
    );

    (command_functions.join("\n\n"), cli_code)
}

mod builtins;
mod error;
#[cfg(feature = "exec")]
pub mod exec;

mod cast;
mod context;
mod mapping;
mod sanitize;

pub use error::{Diagnostic, DiagnosticSeverity, ErrorCollector, Span, TranspilerError};

#[derive(Debug, Error)]
pub enum TranspileError {
    #[error(transparent)]
    Ast(#[from] AstError),
    #[error(transparent)]
    Lookup(#[from] LookupError),
    #[error(transparent)]
    File(#[from] FileError),
}

fn transpile_sources(sources: Vec<Source>) -> Result<Vec<TranspileOutput>, TranspileError> {
    let asts = sources
        .into_iter()
        .map(|s| s.try_into_ast())
        .collect::<Result<Vec<_>, _>>()?;

    transpile_asts(asts)
}

fn transpile_asts(asts: Vec<Ast>) -> Result<Vec<TranspileOutput>, TranspileError> {
    let segmented = asts.segmented()?;
    let builtins = builtins();
    let predefined = predefined_from(&builtins, builtin_fns());
    let lookup = Context::new(builtins).with(&predefined)?.with(&segmented)?;
    let mut scope = Scope::default();
    scope.set_lookup(lookup.lookup.clone());

    transpile_segmented(&segmented, &lookup, &mut scope)
}

struct TypeFileContent<'a> {
    pub ty: &'a TypeDecl,
    pub fns: Vec<&'a FnDecl>,
}

struct ExtensionFileContent<'a> {
    pub elem: &'a TypeElement,
    pub fns: Vec<&'a FnDecl>,
}

fn transpile_segmented(
    segmented: &SegmentedAsts,
    ctx: &Context,
    scope: &mut Scope,
) -> Result<Vec<TranspileOutput>, TranspileError> {
    let mut main_errors = ErrorCollector::new();
    let mut cmd_errors = ErrorCollector::new();
    #[derive(Hash, PartialEq, Eq, Deref, From, Display)]
    struct ModuleName(Box<str>);
    fn module_name(ident: &TypeIdent) -> ModuleName {
        ident.as_str().to_case(Case::Snake).into_boxed_str().into()
    }

    fn extension_module_name(ty: &TypeElement) -> ModuleName {
        extension_name(ty)
            .to_ascii_lowercase()
            .into_boxed_str()
            .into()
    }

    fn add_extension_module<'a>(
        extensions: &mut HashMap<ModuleName, ExtensionFileContent<'a>>,
        func: &'a ToplevelItem<FnDecl>,
        elem: &'a TypeElement,
    ) {
        let content = extensions
            .entry(extension_module_name(elem))
            .or_insert_with(|| ExtensionFileContent {
                elem,
                fns: Vec::new(),
            });
        content.fns.push(&func.item);
    }

    let mut type_files: HashMap<ModuleName, TypeFileContent> = HashMap::new();

    for ty in &segmented.types {
        if let Some(duplicate) = type_files.insert(
            module_name(ty.ident()),
            TypeFileContent {
                ty,
                fns: Vec::new(),
            },
        ) {
            panic!(
                "File collision for types: {} and {}",
                ty.item.ident(),
                duplicate.ty.ident()
            );
        }
    }

    let mut toplevel_functions = Vec::new();
    let mut extensions: HashMap<ModuleName, ExtensionFileContent> = HashMap::new();
    for func in &segmented.functions {
        if let Some(receiver) = func.signature.receiver() {
            let elem = &receiver.param_type;
            let base_type_ident = match elem {
                TypeElement::Plain(ty) => &ty.ident,
                TypeElement::Parametric(ty) => &ty.base_type,
                _ => {
                    add_extension_module(&mut extensions, func, elem);
                    continue;
                }
            };
            match type_files.get_mut(&module_name(base_type_ident)) {
                Some(content) => content.fns.push(&func.item),
                None => {
                    add_extension_module(&mut extensions, func, elem);
                }
            }
        } else {
            toplevel_functions.push(func);
        }
    }

    let type_files = type_files;
    let toplevel_functions = toplevel_functions
        .iter()
        .map(|func| func.transpile(ctx, scope, &mut main_errors))
        .collect::<Vec<_>>()
        .join("\n\n");
    let toplevel_functions = toplevel_functions.trim();

    let tests = transpile_tests(segmented, ctx, scope, &mut main_errors);

    let modules = type_files
        .keys()
        .chain(extensions.keys())
        .map(|id| sanitize_name(id))
        .map(|mod_name| format!("mod {mod_name};\npub use self::{mod_name}::*;"))
        .collect::<Vec<_>>()
        .join("\n");
    let modules = modules.trim();

    let main = segmented
        .main
        .as_ref()
        .map(|main| {
            let main_errors_ref = &mut main_errors;
            transpile!(
                ctx,
                scope,
                main_errors_ref,
                "pub(crate) fn __main__() {}",
                main.body
            )
        })
        .unwrap_or_default();

    let (cmds, cli_main) = if !segmented.cmds.is_empty() {
        let (command_functions, cli_code) =
            generate_cli_structure(&segmented.cmds, ctx, scope, &mut cmd_errors);
        (command_functions, cli_code)
    } else {
        (
            String::new(),
            if segmented.main.is_some() {
                "pub(crate) fn __cli_main() { unreachable!(\"This is not a CLI app.\") }".to_owned()
            } else {
                String::new()
            },
        )
    };

    let has_cli_commands = !segmented.cmds.is_empty();
    let cli_flag = if has_cli_commands {
        "pub(crate) const __HAS_CLI_COMMANDS: bool = true;"
    } else {
        "pub(crate) const __HAS_CLI_COMMANDS: bool = false;"
    };

    let lib = TranspileOutput {
        file_name: galvan_module!("rs").into(),
        content: format!(
            "extern crate galvan; #[allow(unused_imports)] pub(crate) use ::galvan::std::*;\n pub(crate) mod {} {{\n{}\nuse crate::*;\n{}\n{}\n{}\n}}",
            galvan_module!(),
            SUPPRESS_WARNINGS,
            cli_flag,
            [modules, toplevel_functions, &main, &cmds, &tests].join("\n\n"),
            cli_main
        )
        .into(),
    };

    let type_files = type_files
        .iter()
        .map(|(k, v)| TranspileOutput {
            file_name: format!("{k}.rs").into(),
            content: [
                "use crate::*;",
                &v.ty.transpile(ctx, scope, &mut main_errors),
                &transpile_member_functions(v.ty, &v.fns, ctx, scope, &mut main_errors),
            ]
            .join("\n\n")
            .trim()
            .into(),
        })
        .collect_vec();

    let extension_files = extensions
        .iter()
        .map(|(k, v)| TranspileOutput {
            file_name: format!("{k}.rs").into(),
            content: [
                "use crate::*;",
                &transpile_extension_functions(v.elem, &v.fns, ctx, scope, &mut main_errors),
            ]
            .join("\n\n")
            .trim()
            .into(),
        })
        .collect_vec();

    // Output any collected warnings
    for diagnostic in main_errors.diagnostics() {
        match diagnostic.severity {
            DiagnosticSeverity::Error => {
                println!("cargo::error={}", diagnostic.message);
                std::process::exit(1);
            }
            DiagnosticSeverity::Warning => {
                println!("cargo::warning={}", diagnostic.message);
            }
            _ => {}
        }
    }

    Ok(type_files
        .into_iter()
        .chain(extension_files.into_iter())
        .chain(iter::once(lib))
        .collect())
}

fn transpile_tests(
    segmented_asts: &SegmentedAsts,
    ctx: &Context,
    scope: &mut Scope,
    errors: &mut ErrorCollector,
) -> String {
    fn test_name<'a>(desc: &Option<StringLiteral>) -> Cow<'a, str> {
        desc.as_ref().map_or("test".into(), |desc| {
            let snake = desc
                .as_str()
                .trim_matches('\"')
                .to_case(Case::Snake)
                .replace(|c: char| !c.is_ascii_alphanumeric(), "_");

            let snake = if snake.starts_with(|c: char| c.is_ascii_digit()) {
                format!("test_{}", snake)
            } else {
                snake
            };

            if snake.ends_with(|c: char| c.is_ascii_digit()) {
                format!("{}_", snake).into()
            } else {
                snake.into()
            }
        })
    }

    let mut by_name: HashMap<Cow<'_, str>, Vec<&TestDecl>> = HashMap::new();
    for test in &segmented_asts.tests {
        by_name
            .entry(test_name(&test.item.name))
            .or_default()
            .push(&test.item);
    }

    let resolved_tests = by_name
        .iter()
        .flat_map(|(name, tests)| {
            if tests.len() == 1 {
                vec![(name.clone(), tests[0])]
            } else {
                tests
                    .iter()
                    .enumerate()
                    .map(|(i, &test)| (Cow::from(format!("{}_{}", name, i)), test))
                    .collect_vec()
            }
        })
        .collect_vec();

    if resolved_tests.is_empty() {
        return "".into();
    }

    let test_mod = "#[cfg(test)]\nmod tests {\nuse crate::*;\n".to_owned()
        + resolved_tests
            .iter()
            .map(|t| t.transpile(ctx, scope, errors))
            .collect::<Vec<_>>()
            .join("\n\n")
            .as_str()
        + "\n}";

    test_mod
}

fn transpile_member_functions(
    ty: &TypeDecl,
    fns: &[&FnDecl],
    ctx: &Context,
    scope: &mut Scope,
    errors: &mut ErrorCollector,
) -> String {
    if fns.is_empty() {
        return "".into();
    }

    // Collect generic parameters from the type declaration
    let generics = ty.collect_generics();
    let generic_params = if generics.is_empty() {
        String::new()
    } else {
        // Add ToOwned trait bound to all generic parameters for Galvan's ownership semantics
        let params = generics
            .iter()
            .map(|g| {
                format!(
                    "{}: ToOwned<Owned = {}>",
                    capitalize_generic(g.as_str()),
                    capitalize_generic(g.as_str())
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("<{}>", params)
    };

    // Build the type name with generic parameters
    let type_name = if generics.is_empty() {
        format!("{}", ty.ident())
    } else {
        format!(
            "{}<{}>",
            ty.ident(),
            generics
                .iter()
                .map(|g| capitalize_generic(g.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let transpiled_fns = fns
        .iter()
        .map(|f| {
            // Transpile function but strip generic parameters that clash with impl block generics
            let mut fn_content = f.transpile(ctx, scope, errors);

            // Remove redundant generic parameters from function signatures
            // Look for patterns like "fn name<generic>" and replace with "fn name"
            for generic in &generics {
                let generic_lowercase = generic.as_str();
                let generic_capitalized = capitalize_generic(generic.as_str());
                let fn_name = f.signature.identifier.as_str();

                // Try both the original and capitalized versions of the generic parameter
                for generic_str in [generic_lowercase, generic_capitalized.as_str()] {
                    let pattern_with_generics = format!("fn {}<{}>", fn_name, generic_str);
                    let pattern_without_generics = format!("fn {}", fn_name);

                    if fn_content.contains(&pattern_with_generics) {
                        fn_content =
                            fn_content.replace(&pattern_with_generics, &pattern_without_generics);
                    }
                }
            }

            fn_content
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "impl{} {} {{\n{transpiled_fns}\n}}",
        generic_params, type_name
    )
}

fn transpile_extension_functions(
    ty: &TypeElement,
    fns: &[&FnDecl],
    ctx: &Context,
    scope: &mut Scope,
    errors: &mut ErrorCollector,
) -> String {
    debug_assert_ne!(fns.len(), 0, "Extension functions should not be empty");
    if fns
        .iter()
        .find(|f| f.signature.visibility.kind != VisibilityKind::Inherited)
        .is_some()
    {
        // TODO: Add proper error handling for invalid member function visibility
        return String::new();
    }

    let trait_name = extension_name(&ty);
    let fn_signatures = fns
        .iter()
        .map(|f| FnSignature {
            visibility: Visibility::private(),
            ..f.signature.clone()
        })
        .map(|s| s.transpile(ctx, scope, errors))
        .collect::<Vec<_>>()
        .join(";\n")
        + ";";
    let transpiled_fns = fns
        .iter()
        .map(|f| f.transpile(ctx, scope, errors))
        .map(|s| s.strip_prefix("pub(crate) ").unwrap().to_owned())
        .collect::<Vec<_>>()
        .join("\n\n");

    // Handle generic types specially
    match ty {
        TypeElement::Generic(generic_ty) => {
            let generic_param = capitalize_generic(generic_ty.ident.as_str());
            // Extract where clause from the first function, but only include constraints for the impl-level generic (A)
            let where_clause = fns
                .first()
                .and_then(|f| f.signature.where_clause.as_ref())
                .map(|wc| {
                    let generic_param_copy = generic_param.clone();
                    let impl_constraints = wc
                        .bounds
                        .iter()
                        .flat_map(|bound| {
                            let trait_bounds = bound
                                .bounds
                                .iter()
                                .map(|b| b.as_str())
                                .collect::<Vec<_>>()
                                .join(" + ");
                            let generic_param_ref = generic_param_copy.clone();
                            bound.type_params.iter().filter_map(move |p| {
                                let capitalized = capitalize_generic(p.as_str());
                                // Only include constraints for the trait's generic parameter
                                if capitalized == generic_param_ref {
                                    Some(format!("{}: {}", capitalized, trait_bounds))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect::<Vec<_>>();

                    if impl_constraints.is_empty() {
                        String::new()
                    } else {
                        format!(" where {}", impl_constraints.join(", "))
                    }
                })
                .unwrap_or_default();

            // Strip trait-level generic parameters from function signatures
            let fn_signatures_clean = fn_signatures
                .replace(&format!("<{}, ", generic_param), "<")
                .replace(&format!(", {}>", generic_param), ">")
                .replace(&format!("<{}>", generic_param), "")
                .replace(&format!(" where {}: ", generic_param), " where Self: ");
            let transpiled_fns_clean = transpiled_fns
                .replace(&format!("<{}, ", generic_param), "<")
                .replace(&format!(", {}>", generic_param), ">")
                .replace(&format!("<{}>", generic_param), "")
                .replace(&format!(" where {}: ", generic_param), " where Self: ");

            format!(
                "
                pub trait {}<{}> {{
                    {}
                }}

                impl<{}> {}<{}> for {}{} {{
                    {}
                }}
                ",
                trait_name,
                generic_param,
                fn_signatures_clean,
                generic_param,
                trait_name,
                generic_param,
                generic_param,
                where_clause,
                transpiled_fns_clean
            )
        }
        _ => {
            transpile! {ctx, scope, errors,
                "
                pub trait {trait_name} {{
                    {fn_signatures}
                }}

                impl {trait_name} for {} {{
                    {transpiled_fns}
                }}
                ", ty
            }
        }
    }
}

fn extension_name(ty: &TypeElement) -> String {
    fn escaped_name(ty: &TypeElement) -> String {
        match ty {
            TypeElement::Plain(ty) => ty.ident.as_str().to_case(Case::UpperCamel),
            TypeElement::Tuple(ty) => format!(
                "Tuple_{}",
                ty.elements
                    .iter()
                    .map(escaped_name)
                    .collect::<Vec<_>>()
                    .join("_")
            ),
            TypeElement::Result(ty) => format!(
                "Result_{}_{}",
                escaped_name(&ty.success),
                ty.error.as_ref().map_or("".into(), escaped_name)
            ),
            TypeElement::Optional(ty) => format!("Option_{}_Ext", escaped_name(&ty.inner)),
            TypeElement::Dictionary(ty) => {
                format!("Dict_{}_{}", escaped_name(&ty.key), escaped_name(&ty.value))
            }
            TypeElement::OrderedDictionary(ty) => format!(
                "OrderedDict_{}_{}",
                escaped_name(&ty.key),
                escaped_name(&ty.value)
            ),
            TypeElement::Array(ty) => format!("Array_{}", escaped_name(&ty.elements)),
            TypeElement::Set(ty) => format!("Set_{}", escaped_name(&ty.elements)),
            TypeElement::Generic(ty) => {
                format!("Generic_{}", capitalize_generic(ty.ident.as_str()))
            }
            TypeElement::Parametric(ty) => {
                let base_type_item = BasicTypeItem {
                    ident: ty.base_type.clone(),
                    span: galvan_ast::Span::default(),
                };
                let base = escaped_name(&TypeElement::Plain(base_type_item));
                let args = ty
                    .type_args
                    .iter()
                    .map(escaped_name)
                    .collect::<Vec<_>>()
                    .join("_");
                format!("{}_{}", base, args)
            }
            TypeElement::Closure(ty) => {
                let args = ty
                    .parameters
                    .iter()
                    .map(escaped_name)
                    .collect::<Vec<_>>()
                    .join("_");
                format!("Closure_{}__{}", args, escaped_name(&ty.return_ty))
            }
            TypeElement::Void(_) => format!("Void"),
            TypeElement::Infer(_) => format!("Infer"),
            TypeElement::Never(_) => format!("Never"),
        }
    }

    escaped_name(ty) + "_Ext"
}

pub struct TranspileOutput {
    pub file_name: Box<str>,
    pub content: Box<str>,
}

pub struct TranspileErrors<'t> {
    pub source: Source,
    pub errors: &'t [TranspileError],
}

impl TranspileErrors<'_> {
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn transpile(sources: Vec<Source>) -> Result<Vec<TranspileOutput>, TranspileError> {
    transpile_sources(sources)
}

mod transpile_item;
mod type_inference;

trait Transpile {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String;
}

trait Punctuated {
    fn punctuation() -> &'static str;
}

mod macros {
    macro_rules! transpile {
        ($ctx:ident, $scope:ident, $errors:ident, $string:expr, $($items:expr),*$(,)?) => {
            format!($string, $(($items).transpile($ctx, $scope, $errors)),*)
        };

        // Temporary backward compatibility - creates a local temp ErrorCollector
        ($ctx:ident, $scope:ident, $string:expr, $($items:expr),*$(,)?) => {
            {
                let mut _temp_errors = crate::ErrorCollector::new();
                format!($string, $(($items).transpile($ctx, $scope, &mut _temp_errors)),*)
            }
        };
    }

    macro_rules! impl_transpile {
        ($ty:ty, $string:expr, $($field:tt),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(&self, _ctx: &crate::Context, _scope: &mut crate::Scope, _errors: &mut crate::ErrorCollector) -> String {
                    crate::macros::transpile!(_ctx, _scope, _errors, $string, $(self.$field),*)
                }
            }
        };

        // Temporary backward compatibility
        ($ty:ty, $old_signature:expr, $string:expr, $($field:tt),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(&self, _ctx: &crate::Context, _scope: &mut crate::Scope, _errors: &mut crate::ErrorCollector) -> String {
                    crate::macros::transpile!(_ctx, _scope, _errors, $string, $(self.$field),*)
                }
            }
        };
    }

    #[allow(unused_macros)]
    macro_rules! impl_transpile_fn {
        ($ty:ty, $string:expr, $($fun:ident),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(&self, ctx: &crate::Context, scope: &mut crate::Scope) -> String {
                    crate::macros::transpile!(ctx, scope, $string, $(self.$fun()),*)
                }
            }
        };
    }

    macro_rules! impl_transpile_match {
        ($ty:ty, $($case:pat_param => ($($args:expr),+)),+$(,)?) => {
            impl crate::Transpile for $ty {
                #[deny(bindings_with_variant_name)]
                #[deny(unreachable_patterns)]
                #[deny(non_snake_case)]
                fn transpile(&self, ctx: &crate::Context, scope: &mut crate::Scope, errors: &mut crate::ErrorCollector) -> String {
                    use $ty::*;
                    match self {
                        $($case => crate::macros::transpile!(ctx, scope, errors, $($args),+),)+
                    }
                }
            }
        };
    }

    macro_rules! impl_transpile_variants {
        ($ty:ty; $($case:ident$(,)?)+) => {
            impl crate::Transpile for $ty {
                #[deny(bindings_with_variant_name)]
                #[deny(unreachable_patterns)]
                #[deny(non_snake_case)]
                fn transpile(&self, ctx: &crate::Context, scope: &mut crate::Scope, errors: &mut crate::ErrorCollector) -> String {
                    use $ty::*;
                    match self {
                        $($case(inner) => inner.transpile(ctx, scope, errors),)+
                    }
                }
            }
        };
    }

    macro_rules! punct {
        ($string:expr, $($ty:ty),+) => {
            $(impl Punctuated for $ty {
                fn punctuation() -> &'static str {
                    $string
                }
            })+
            $(impl Punctuated for &$ty {
                fn punctuation() -> &'static str {
                    $string
                }
            })+
        };
    }

    pub(crate) use {
        impl_transpile, impl_transpile_match, impl_transpile_variants, punct, transpile,
    };
}

use crate::builtins::builtins;
use crate::context::{predefined_from, Context};
use crate::macros::transpile;
use crate::sanitize::sanitize_name;
use macros::punct;

punct!(
    ", ",
    TypeElement,
    TupleTypeMember,
    Param,
    ConstructorCallArg,
    ClosureParameter,
    DictLiteralElement
);
punct!(",\n", StructTypeMember, EnumTypeMember);
punct!("\n\n", RootItem, FnDecl);
punct!(";\n", Statement);

impl<T> Transpile for Vec<T>
where
    T: Transpile + Punctuated,
{
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        self.as_slice().transpile(ctx, scope, errors)
    }
}

impl<T> Transpile for [T]
where
    T: Transpile + Punctuated,
{
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let punct = T::punctuation();
        self.iter()
            .map(|e| e.transpile(ctx, scope, errors))
            .reduce(|acc, e| format!("{acc}{punct}{e}"))
            .unwrap_or_else(String::new)
    }
}

impl<T> Transpile for Option<Vec<T>>
where
    T: Transpile + Punctuated,
{
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        self.as_ref()
            .map_or_else(String::new, |v| v.transpile(ctx, scope, errors))
    }
}

impl Transpile for &str {
    fn transpile(
        &self,
        _ctx: &Context,
        _scope: &mut Scope,
        _errors: &mut ErrorCollector,
    ) -> String {
        self.to_string()
    }
}

impl Transpile for String {
    fn transpile(
        &self,
        _ctx: &Context,
        _scope: &mut Scope,
        _errors: &mut ErrorCollector,
    ) -> String {
        self.to_owned()
    }
}

impl<T> Transpile for Box<T>
where
    T: Transpile,
{
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        self.as_ref().transpile(ctx, scope, errors)
    }
}
