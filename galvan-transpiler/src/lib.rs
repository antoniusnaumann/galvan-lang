use convert_case::{Case, Casing};
use derive_more::{Deref, Display, From};
use galvan_ast::*;
use galvan_files::{FileError, Source};
use galvan_resolver::{LookupError, Scope};
use itertools::Itertools;
use std::borrow::Cow;
use std::collections::HashMap;
use std::iter;
use thiserror::Error;

static SUPPRESS_WARNINGS: &str = "#![allow(unused_imports)]\n#![allow(dead_code)]";

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
#[cfg(feature = "exec")]
pub mod exec;

mod context;
mod mapping;
mod sanitize;

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
    let predefined = predefined_from(&builtins);
    let lookup = Context::new(builtins).with(&predefined)?.with(&segmented)?;
    let mut scope = Scope::default();

    transpile_segmented(&segmented, &lookup, &mut scope)
}

struct TypeFileContent<'a> {
    pub ty: &'a TypeDecl,
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
    for func in &segmented.functions {
        if let Some(receiver) = func.signature.receiver() {
            let TypeElement::Plain(ty) = &receiver.param_type else {
                todo!("Allow extending complex types")
            };
            let content = type_files.get_mut(&module_name(&ty.ident)).expect(
                "TODO: Handle error for member functions that refer to types that are not declared",
            );
            content.fns.push(&func.item);
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

    let tests = transpile_tests(&segmented, ctx, scope);

    let modules = type_files
        .keys()
        .map(|id| sanitize_name(id))
        .map(|mod_name| format!("mod {mod_name};\npub use self::{mod_name}::*;"))
        .collect::<Vec<_>>()
        .join("\n");
    let modules = modules.trim();

    let main = segmented
        .main
        .as_ref()
        .map(|main| transpile!(ctx, scope, "pub(crate) fn __main__() {{\n{}\n}}", main.body))
        .unwrap_or_default();

    let lib = TranspileOutput {
        file_name: galvan_module!("rs").into(),
        content: format!(
            "extern crate galvan; pub(crate) use ::galvan::std::*;\n pub(crate) mod {} {{\n{}\nuse crate::*;\n{}\n}}",
            galvan_module!(),
            SUPPRESS_WARNINGS,
            [modules, toplevel_functions, &main, &tests].join("\n\n")
        )
        .into(),
    };

    let type_files = type_files.iter().map(|(k, v)| TranspileOutput {
        file_name: format!("{k}.rs").into(),
        content: [
            "use crate::*;",
            &v.ty.transpile(ctx, scope),
            &transpile_member_functions(v.ty.ident(), &v.fns, ctx, scope),
        ]
        .join("\n\n")
        .trim()
        .into(),
    });

    Ok(type_files.chain(iter::once(lib)).collect())
}

fn transpile_tests(segmented_asts: &SegmentedAsts, ctx: &Context, scope: &mut Scope) -> String {
    fn test_name<'a>(desc: &Option<StringLiteral>) -> Cow<'a, str> {
        desc.as_ref().map_or("test".into(), |desc| {
            let snake = desc.as_str().trim_matches('\"').to_case(Case::Snake);
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
        ($ty:ty, $string:expr, $($field:ident),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(&self, ctx: &crate::Context, scope: &mut crate::Scope) -> String {
                    crate::macros::transpile!(ctx, scope, $string, $(self.$field),*)
                }
            }
        };
    }

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
        impl_transpile, impl_transpile_fn, impl_transpile_match, impl_transpile_variants, punct,
        transpile,
    };
}
use crate::builtins::builtins;
use crate::context::{predefined_from, Context};
use crate::macros::transpile;
use crate::mapping::Mapping;
use crate::sanitize::sanitize_name;
use macros::punct;

punct!(
    ", ",
    TypeElement,
    TupleTypeMember,
    Param,
    FunctionCallArg,
    ConstructorCallArg,
    ClosureArgument,
    DictLiteralElement
);
punct!(",\n", StructTypeMember);
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
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        self.to_string()
    }
}

impl Transpile for String {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
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
