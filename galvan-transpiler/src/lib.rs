use galvan_ast::*;
use galvan_files::Source;

use thiserror::Error;

pub(crate) use galvan_resolver::LookupContext;
use galvan_resolver::LookupError;

// TODO: Maybe use something like https://crates.io/crates/ruast to generate the Rust code in a more reliable way

/// Name of the generated rust module that exports all public items from all galvan files in this crate
#[macro_export]
macro_rules! galvan_module {
    () => {
        "galvan_module.rs"
    };
}

#[cfg(feature = "exec")]
pub mod exec;

#[derive(Debug, Error)]
pub enum TranspileError {
    #[error(transparent)]
    Ast(#[from] AstError),
    #[error(transparent)]
    Lookup(#[from] LookupError),
}

fn transpile_source(source: Source) -> Result<String, TranspileError> {
    let ast = source.try_into_ast()?;
    // TODO: Declare extern types in standard library instead of hardcoding them here
    let predefined = Source::from_string(
        "
        pub type Int
        pub type Float 
        pub type String
        ",
    )
    .try_into_ast()
    .expect("Failed to parse predefined types");
    let asts = [ast, predefined];
    let lookup = LookupContext::new(&asts)?;

    Ok(asts[0].transpile(&lookup))
}

#[derive(Debug)]
pub struct Transpilation {
    pub source: Source,
    pub transpiled: Result<String, TranspileError>,
}

pub struct SuccessfulTranspilation {
    pub source: Source,
    pub transpiled: String,
}

pub struct FailedTranspilation {
    pub source: Source,
    pub errors: TranspileError,
}

impl From<Transpilation> for Result<SuccessfulTranspilation, FailedTranspilation> {
    fn from(value: Transpilation) -> Self {
        match value.transpiled {
            Ok(transpiled) => Ok(SuccessfulTranspilation {
                source: value.source,
                transpiled,
            }),
            Err(errors) => Err(FailedTranspilation {
                source: value.source,
                errors,
            }),
        }
    }
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

pub fn transpile(source: Source) -> Transpilation {
    Transpilation {
        source: source.clone(),
        transpiled: transpile_source(source),
    }
}

mod transpile_item {
    mod body;
    mod fn_decl;
    mod ident;
    mod r#struct;
    mod task;
    mod toplevel;
    mod r#type;
    mod visibility;
}

trait Transpile {
    fn transpile(&self, lookup: &LookupContext) -> String;
}

trait Punctuated {
    fn punctuation() -> &'static str;
}

mod macros {
    macro_rules! transpile {
        ($lookup:ident, $string:expr, $($items:expr),*$(,)?) => {
            format!($string, $(($items).transpile($lookup)),*)
        };
    }

    macro_rules! impl_transpile {
        ($ty:ty, $string:expr, $($field:ident),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(&self, lookup: &crate::LookupContext) -> String {
                    crate::macros::transpile!(lookup, $string, $(self.$field),*)
                }
            }
        };
    }

    macro_rules! impl_transpile_fn {
        ($ty:ty, $string:expr, $($fun:ident),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(&self, lookup: &crate::LookupContext) -> String {
                    crate::macros::transpile!(lookup, $string, $(self.$fun()),*)
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
                fn transpile(&self, lookup: &crate::LookupContext) -> String {
                    use $ty::*;
                    match self {
                        $($case => crate::macros::transpile!(lookup, $($args),+),)+
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
                fn transpile(&self, lookup: &crate::LookupContext) -> String {
                    use $ty::*;
                    match self {
                        $($case(inner) => inner.transpile(lookup),)+
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
        };
    }

    pub(crate) use {
        impl_transpile, impl_transpile_fn, impl_transpile_match, impl_transpile_variants, punct,
        transpile,
    };
}
use macros::punct;

punct!(", ", TypeElement, TupleTypeMember, Param);
punct!(",\n", StructTypeMember);
punct!("\n\n", RootItem);
// punct!(";\n", Statement);

impl<T> Transpile for Vec<T>
where
    T: Transpile + Punctuated,
{
    fn transpile(&self, lookup: &LookupContext) -> String {
        let punct = T::punctuation();
        self.into_iter()
            .map(|e| e.transpile(lookup))
            .reduce(|acc, e| format!("{acc}{punct}{e}"))
            .unwrap_or_else(String::new)
    }
}
