use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::iter;

use convert_case::{Case, Casing};
use derive_more::{Deref, Display, From};
use itertools::Itertools;
use thiserror::Error;

use galvan_ast::*;
use galvan_files::{FileError, Source};
use galvan_hir::hir::{HirCmd, HirFunction, HirMain, HirMainKind, HirModule, HirTest};
use galvan_hir::typecheck::typecheck;
use galvan_into_ast::{AstError, SegmentAst, SourceIntoAst};
use galvan_resolver::LookupError;

use crate::codegen::{transpile_function, transpile_main, transpile_signature, transpile_test};

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

struct CliArguments {
    fields: Vec<String>,
    values: Vec<String>,
}

fn cli_arguments(
    parameters: &ParamList,
    source: &Source,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> CliArguments {
    let mut fields = Vec::new();
    let mut values = Vec::new();

    for param in &parameters.params {
        let field_name = param.identifier.as_str();
        let param_type = param.param_type.transpile(ctx, errors);
        let help_text = extract_param_doc_comment(source.content(), param);
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
            (None, None) => format!("#[arg(long = \"{}\")]", field_name),
        };

        fields.push(format!(
            "    {clap_attr}\n    pub {field_name}: {param_type}"
        ));
        values.push(field_name.to_owned());
    }

    CliArguments { fields, values }
}

/// Generate CLI structure with top-level arguments and subcommands.
fn generate_cli_structure(
    commands: &[HirCmd],
    main: Option<&HirMain>,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> (String, String) {
    let mut command_functions = Vec::new();
    let mut subcommand_variants = Vec::new();
    let mut subcommand_args = Vec::new();
    let mut match_arms = Vec::new();

    for cmd in commands {
        let cmd_name = cmd.signature.identifier.as_str();
        let cmd_name_pascal = cmd_name.to_case(Case::Pascal);

        // Generate the command function
        let signature = cmd.signature.transpile(ctx, errors);
        let body = cmd.body.transpile(ctx, errors);
        command_functions.push(format!("{signature} {body}"));

        // Generate args struct for this command
        let arguments = cli_arguments(&cmd.signature.parameters, &cmd.source, ctx, errors);

        let args_struct = if arguments.fields.is_empty() {
            format!(
                "#[derive(clap::Args, Debug)]\nstruct {}Args {{}}",
                cmd_name_pascal
            )
        } else {
            format!(
                "#[derive(clap::Args, Debug)]\nstruct {}Args {{\n{}\n}}",
                cmd_name_pascal,
                arguments.fields.join(",\n")
            )
        };

        subcommand_args.push(args_struct);

        // Extract doc comment for the command itself
        let cmd_help = extract_doc_comment(cmd.source.content(), &cmd.span);

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
        let function_call = if arguments.values.is_empty() {
            format!("{}()", cmd_name)
        } else {
            format!(
                "{}({})",
                cmd_name,
                arguments
                    .values
                    .iter()
                    .map(|field| format!("args.{field}"))
                    .join(", ")
            )
        };

        match_arms.push(format!(
            "        Some(Commands::{}(args)) => {},",
            cmd_name_pascal, function_call
        ));
    }

    let main_arguments = match main {
        Some(HirMain {
            kind: HirMainKind::Command { signature },
            source,
            ..
        }) => Some(cli_arguments(&signature.parameters, source, ctx, errors)),
        _ => None,
    };
    let main_call = match &main_arguments {
        Some(arguments) if arguments.values.is_empty() => "__main_command()".to_owned(),
        Some(arguments) => format!("__main_command({})", arguments.values.join(", ")),
        None if main.is_some() => "__main__()".to_owned(),
        None => "{}".to_owned(),
    };
    let main_fields = main_arguments
        .as_ref()
        .map(|arguments| arguments.fields.join(",\n"))
        .unwrap_or_default();
    let main_bindings = main_arguments
        .as_ref()
        .map(|arguments| arguments.values.join(", "))
        .unwrap_or_default();
    let cli_bindings = if main_bindings.is_empty() {
        "command".to_owned()
    } else {
        format!("{main_bindings}, command")
    };

    let cli_code = if commands.is_empty() {
        format!(
            r#"
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {{
{main_fields}
}}

pub(crate) fn __cli_main() {{
    let cli = Cli::parse();
    let Cli {{ {} }} = cli;
    {main_call};
}}
"#,
            main_bindings
        )
    } else {
        let main_fields = if main_fields.is_empty() {
            String::new()
        } else {
            format!("{main_fields},")
        };
        format!(
            r#"
use clap::{{Parser, Subcommand, Args}};

#[derive(Parser)]
#[command(author, version, about, long_about = None, subcommand_negates_reqs = true)]
struct Cli {{
{main_fields}
    #[command(subcommand)]
    command: Option<Commands>,
}}

#[derive(Subcommand)]
enum Commands {{
{}
}}

{}

pub(crate) fn __cli_main() {{
    let Cli {{ {cli_bindings} }} = Cli::parse();
    match command {{
{}
        None => {main_call},
    }}
}}
"#,
            subcommand_variants.join(",\n"),
            subcommand_args.join("\n\n"),
            match_arms.join("\n")
        )
    };

    (command_functions.join("\n\n"), cli_code)
}

mod codegen;
#[cfg(feature = "exec")]
pub mod exec;

mod context;
mod sanitize;

pub use galvan_hir::error::{
    Diagnostic, DiagnosticSeverity, ErrorCollector, Span, TranspilerError,
};

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
    let (module, mut errors) = typecheck(segmented)?;

    let builtins = builtins();
    let predefined = predefined_from(&builtins, builtin_fns());
    let mut ctx = Context::new(builtins);
    ctx = ctx.with(&predefined)?;
    for ty in &module.types {
        ctx.lookup.types.insert(ty.item.ident().clone(), ty);
    }

    transpile_module(&module, &ctx, &mut errors)
}

struct TypeFileContent<'a> {
    pub ty: &'a TypeDecl,
    pub fns: Vec<&'a HirFunction>,
}

struct ExtensionFileContent<'a> {
    pub elem: &'a TypeElement,
    pub fns: Vec<&'a HirFunction>,
}

fn transpile_uses(uses: &[ToplevelItem<UseDecl>]) -> String {
    uses.iter()
        .map(|use_decl| {
            let path = crate::sanitize::sanitize_path(&use_decl.path);
            if use_decl.path.segments.len() == 1 {
                format!("use {path}::*;")
            } else {
                format!("use {path};")
            }
        })
        .join("\n")
}

fn transpile_module(
    module: &HirModule,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> Result<Vec<TranspileOutput>, TranspileError> {
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
        func: &'a HirFunction,
        elem: &'a TypeElement,
    ) {
        let content = extensions
            .entry(extension_module_name(elem))
            .or_insert_with(|| ExtensionFileContent {
                elem,
                fns: Vec::new(),
            });
        content.fns.push(func);
    }

    let mut type_files: HashMap<ModuleName, TypeFileContent> = HashMap::new();

    for ty in &module.types {
        if let Some(duplicate) = type_files.insert(
            module_name(ty.item.ident()),
            TypeFileContent {
                ty: &ty.item,
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
    for func in &module.functions {
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
                Some(content) => content.fns.push(func),
                None => {
                    add_extension_module(&mut extensions, func, elem);
                }
            }
        } else {
            toplevel_functions.push(func);
        }
    }

    let no_generics = HashSet::new();
    let imports = transpile_uses(&module.uses);
    let type_files = type_files;
    let toplevel_functions = toplevel_functions
        .iter()
        .map(|func| transpile_function(func, ctx, errors, &no_generics))
        .collect::<Vec<_>>()
        .join("\n\n");
    let toplevel_functions = toplevel_functions.trim();

    let tests = transpile_tests(&module.tests, &imports, ctx, errors);

    let modules = type_files
        .keys()
        .chain(extensions.keys())
        .map(|id| sanitize_name(id))
        .map(|mod_name| format!("mod {mod_name};\npub use self::{mod_name}::*;"))
        .collect::<Vec<_>>()
        .join("\n");
    let modules = modules.trim();

    let main = module
        .main
        .as_ref()
        .map(|main| transpile_main(main, ctx, errors))
        .unwrap_or_else(|| {
            if module.cmds.is_empty() {
                String::new()
            } else {
                "pub(crate) fn __main__() { unreachable!(\"No default main command\") }".to_owned()
            }
        });

    let has_command_main = module
        .main
        .as_ref()
        .is_some_and(|main| matches!(&main.kind, HirMainKind::Command { .. }));
    let (cmds, cli_main) = if !module.cmds.is_empty() || has_command_main {
        let (command_functions, cli_code) =
            generate_cli_structure(&module.cmds, module.main.as_ref(), ctx, errors);
        (command_functions, cli_code)
    } else {
        (
            String::new(),
            if module.main.is_some() {
                "pub(crate) fn __cli_main() { unreachable!(\"This is not a CLI app.\") }".to_owned()
            } else {
                String::new()
            },
        )
    };

    let has_cli_commands = !module.cmds.is_empty() || has_command_main;
    let cli_flag = if has_cli_commands {
        "pub(crate) const __HAS_CLI_COMMANDS: bool = true;"
    } else {
        "pub(crate) const __HAS_CLI_COMMANDS: bool = false;"
    };

    let lib = TranspileOutput {
        file_name: galvan_module!("rs").into(),
        content: format!(
            "extern crate galvan; #[allow(unused_imports)] pub(crate) use ::galvan::std::*;\n pub(crate) mod {} {{\n{}\nuse crate::*;\n{}\n{}\n{}\n{}\n}}",
            galvan_module!(),
            SUPPRESS_WARNINGS,
            imports,
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
                &imports,
                &v.ty.transpile(ctx, errors),
                &transpile_member_functions(v.ty, &v.fns, ctx, errors),
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
                &imports,
                &transpile_extension_functions(v.elem, &v.fns, ctx, errors),
            ]
            .join("\n\n")
            .trim()
            .into(),
        })
        .collect_vec();

    // Output any collected warnings
    for diagnostic in errors.diagnostics() {
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
    tests: &[HirTest],
    imports: &str,
    ctx: &Context,
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

    let mut by_name: HashMap<Cow<'_, str>, Vec<&HirTest>> = HashMap::new();
    for test in tests {
        by_name.entry(test_name(&test.name)).or_default().push(test);
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

    let test_mod = format!("#[cfg(test)]\nmod tests {{\nuse crate::*;\n{imports}\n")
        + resolved_tests
            .iter()
            .map(|(name, test)| transpile_test(name, test, ctx, errors))
            .collect::<Vec<_>>()
            .join("\n\n")
            .as_str()
        + "\n}";

    test_mod
}

fn transpile_member_functions(
    ty: &TypeDecl,
    fns: &[&HirFunction],
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    if fns.is_empty() {
        return "".into();
    }

    // Collect generic parameters from the type declaration; they are
    // declared on the impl block and skipped on the member functions
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
        .map(|f| transpile_function(f, ctx, errors, &generics))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "impl{} {} {{\n{transpiled_fns}\n}}",
        generic_params, type_name
    )
}

fn transpile_extension_functions(
    ty: &TypeElement,
    fns: &[&HirFunction],
    ctx: &Context,
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

    let no_generics = HashSet::new();
    let trait_name = extension_name(&ty);
    let fn_signatures = fns
        .iter()
        .map(|f| FnSignature {
            visibility: Visibility::private(),
            ..f.signature.clone()
        })
        .map(|s| transpile_signature(&s, ctx, errors, &no_generics))
        .collect::<Vec<_>>()
        .join(";\n")
        + ";";
    let transpiled_fns = fns
        .iter()
        .map(|f| transpile_function(f, ctx, errors, &no_generics))
        .map(|s| s.strip_prefix("pub(crate) ").unwrap().to_owned())
        .collect::<Vec<_>>()
        .join("\n\n");

    let mut generics = HashSet::new();
    ty.collect_generics_recursive(&mut generics);

    // Extract where clause from the first function, but only include constraints for the impl-level generic (A)
    // TODO: we should group impl blocks by constraints instead of blindly taking the first where clause
    fn transpile_where_clause(fns: &[&HirFunction], generic_param: &str) -> String {
        fns.first()
            .and_then(|f| f.signature.where_clause.as_ref())
            .map(|wc| {
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
                        bound.type_params.iter().filter_map(move |p| {
                            let capitalized = capitalize_generic(p.as_str());
                            // Only include constraints for the trait's generic parameter
                            if capitalized == generic_param {
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
            .unwrap_or_default()
    }

    // Handle generic types specially
    match ty {
        TypeElement::Generic(generic_ty) => {
            let generic_param = capitalize_generic(generic_ty.ident.as_str());
            let where_clause = transpile_where_clause(fns, &generic_param);

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
                fn_signatures,
                generic_param,
                trait_name,
                generic_param,
                generic_param,
                where_clause,
                transpiled_fns
            )
        }
        _ if !generics.is_empty() => {
            let generics = generics
                .iter()
                .map(|g| capitalize_generic(g.as_str()))
                .join(", ");

            // TODO: we probably need to transpile the where_clause here like above

            transpile! {ctx, errors,
                "
                pub trait {trait_name}<{generics}> {{
                    {fn_signatures}
                }}

                impl <{generics}> {trait_name}<{generics}> for {} {{
                    {transpiled_fns}
                }}
                ", ty
            }
        }
        _ => {
            transpile! {ctx, errors,
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

trait Transpile {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String;
}

trait Punctuated {
    fn punctuation() -> &'static str;
}

mod macros {
    macro_rules! transpile {
        ($ctx:ident, $errors:ident, $string:expr, $($items:expr),*$(,)?) => {
            format!($string, $(($items).transpile($ctx, $errors)),*)
        };
    }

    macro_rules! impl_transpile {
        ($ty:ty, $string:expr, $($field:tt),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(&self, _ctx: &crate::Context, _errors: &mut crate::ErrorCollector) -> String {
                    crate::macros::transpile!(_ctx, _errors, $string, $(self.$field),*)
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
                fn transpile(&self, ctx: &crate::Context, errors: &mut crate::ErrorCollector) -> String {
                    use $ty::*;
                    match self {
                        $($case(inner) => inner.transpile(ctx, errors),)+
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

    #[allow(unused_imports)]
    pub(crate) use {impl_transpile, impl_transpile_variants, punct, transpile};
}

use crate::context::{predefined_from, Context};
use crate::macros::transpile;
use crate::sanitize::sanitize_name;
use galvan_hir::builtins::{builtin_fns, builtins};
use macros::punct;

punct!(", ", TypeElement, TupleTypeMember);
punct!(",\n", StructTypeMember, EnumTypeMember);

impl<T> Transpile for Vec<T>
where
    T: Transpile + Punctuated,
{
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        self.as_slice().transpile(ctx, errors)
    }
}

impl<T> Transpile for [T]
where
    T: Transpile + Punctuated,
{
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let punct = T::punctuation();
        self.iter()
            .map(|e| e.transpile(ctx, errors))
            .reduce(|acc, e| format!("{acc}{punct}{e}"))
            .unwrap_or_else(String::new)
    }
}

impl Transpile for &str {
    fn transpile(&self, _ctx: &Context, _errors: &mut ErrorCollector) -> String {
        self.to_string()
    }
}

impl Transpile for String {
    fn transpile(&self, _ctx: &Context, _errors: &mut ErrorCollector) -> String {
        self.to_owned()
    }
}

impl<T> Transpile for Box<T>
where
    T: Transpile,
{
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        self.as_ref().transpile(ctx, errors)
    }
}
