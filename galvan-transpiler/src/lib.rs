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

mod builtins;
mod error;
#[cfg(feature = "exec")]
pub mod exec;

mod cast;
mod context;
mod mapping;
mod sanitize;

pub use error::{ErrorCollector, TranspilerError, Diagnostic, DiagnosticSeverity, Span};

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
            let TypeElement::Plain(ty) = elem else {
                add_extension_module(&mut extensions, func, elem);
                continue;
            };
            match type_files.get_mut(&module_name(&ty.ident)) {
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
        .map(|func| func.transpile(ctx, scope))
        .collect::<Vec<_>>()
        .join("\n\n");
    let toplevel_functions = toplevel_functions.trim();

    let tests = transpile_tests(segmented, ctx, scope);

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
        .map(|main| transpile!(ctx, scope, "pub(crate) fn __main__() {}", main.body))
        .unwrap_or_default();

    let lib = TranspileOutput {
        file_name: galvan_module!("rs").into(),
        content: format!(
            "extern crate galvan; #[allow(unused_imports)] pub(crate) use ::galvan::std::*;\n pub(crate) mod {} {{\n{}\nuse crate::*;\n{}\n}}",
            galvan_module!(),
            SUPPRESS_WARNINGS,
            [modules, toplevel_functions, &main, &tests].join("\n\n")
        )
        .into(),
    };

    let type_files = type_files
        .iter()
        .map(|(k, v)| TranspileOutput {
            file_name: format!("{k}.rs").into(),
            content: [
                "use crate::*;",
                &v.ty.transpile(ctx, scope),
                &transpile_member_functions(v.ty.ident(), &v.fns, ctx, scope),
            ]
            .join("\n\n")
            .trim()
            .into(),
        })
        .collect_vec();

    let extension_files = extensions.iter().map(|(k, v)| TranspileOutput {
        file_name: format!("{k}.rs").into(),
        content: [
            "use crate::*;",
            &transpile_extension_functions(v.elem, &v.fns, ctx, scope),
        ]
        .join("\n\n")
        .trim()
        .into(),
    });

    Ok(type_files
        .into_iter()
        .chain(extension_files)
        .chain(iter::once(lib))
        .collect())
}

fn transpile_tests(segmented_asts: &SegmentedAsts, ctx: &Context, scope: &mut Scope) -> String {
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
            .map(|t| t.transpile(ctx, scope))
            .collect::<Vec<_>>()
            .join("\n\n")
            .as_str()
        + "\n}";

    test_mod
}

fn transpile_member_functions(
    ty: &TypeIdent,
    fns: &[&FnDecl],
    ctx: &Context,
    scope: &mut Scope,
) -> String {
    if fns.is_empty() {
        return "".into();
    }

    let transpiled_fns = fns
        .iter()
        .map(|f| f.transpile(ctx, scope))
        .collect::<Vec<_>>()
        .join("\n\n");
    transpile!(ctx, scope, "impl {} {{\n{transpiled_fns}\n}}", ty)
}

fn transpile_extension_functions(
    ty: &TypeElement,
    fns: &[&FnDecl],
    ctx: &Context,
    scope: &mut Scope,
) -> String {
    debug_assert_ne!(fns.len(), 0, "Extension functions should not be empty");
    if fns
        .iter()
        .find(|f| f.signature.visibility.kind != VisibilityKind::Inherited)
        .is_some()
    {
        todo!("TRANSPILER ERROR: Member functions for types declared outside of galvan module must have default visibility!");
    }

    let trait_name = extension_name(&ty);
    let fn_signatures = fns
        .iter()
        .map(|f| FnSignature {
            visibility: Visibility::private(),
            ..f.signature.clone()
        })
        .map(|s| s.transpile(ctx, scope))
        .collect::<Vec<_>>()
        .join(";\n")
        + ";";
    let transpiled_fns = fns
        .iter()
        .map(|f| f.transpile(ctx, scope))
        .map(|s| s.strip_prefix("pub(crate) ").unwrap().to_owned())
        .collect::<Vec<_>>()
        .join("\n\n");

    transpile! {ctx, scope,
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
            TypeElement::Generic(_ty) => todo!("Generics are not supported yet!"),
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
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String;
}

trait Punctuated {
    fn punctuation() -> &'static str;
}

mod macros {
    macro_rules! transpile {
        ($ctx:ident, $scope:ident, $string:expr, $($items:expr),*$(,)?) => {
            format!($string, $(($items).transpile($ctx, $scope)),*)
        };
    }

    macro_rules! impl_transpile {
        ($ty:ty, $string:expr, $($field:tt),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(&self, _ctx: &crate::Context, _scope: &mut crate::Scope) -> String {
                    crate::macros::transpile!(_ctx, _scope, $string, $(self.$field),*)
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
                fn transpile(&self, ctx: &crate::Context, scope: &mut crate::Scope) -> String {
                    use $ty::*;
                    match self {
                        $($case => crate::macros::transpile!(ctx, scope, $($args),+),)+
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
                fn transpile(&self, ctx: &crate::Context, scope: &mut crate::Scope) -> String {
                    use $ty::*;
                    match self {
                        $($case(inner) => inner.transpile(ctx, scope),)+
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
        impl_transpile, impl_transpile_match, impl_transpile_variants, punct,
        transpile,
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
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        self.as_slice().transpile(ctx, scope)
    }
}

impl<T> Transpile for [T]
where
    T: Transpile + Punctuated,
{
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let punct = T::punctuation();
        self.iter()
            .map(|e| e.transpile(ctx, scope))
            .reduce(|acc, e| format!("{acc}{punct}{e}"))
            .unwrap_or_else(String::new)
    }
}

impl<T> Transpile for Option<Vec<T>>
where
    T: Transpile + Punctuated,
{
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        self.as_ref()
            .map_or_else(String::new, |v| v.transpile(ctx, scope))
    }
}

impl Transpile for &str {
    fn transpile(&self, _ctx: &Context, _scope: &mut Scope) -> String {
        self.to_string()
    }
}

impl Transpile for String {
    fn transpile(&self, _ctx: &Context, _scope: &mut Scope) -> String {
        self.to_owned()
    }
}

impl<T> Transpile for Box<T>
where
    T: Transpile,
{
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        self.as_ref().transpile(ctx, scope)
    }
}
