extern crate core;

use galvan_ast::*;
use galvan_files::Source;

#[cfg(feature = "exec")]
pub mod exec;

// TODO: This should get its own error type
pub type TranspileError = AstError;
fn transpile_source(source: Source) -> Result<String, TranspileError> {
    let ast = source.try_into_ast()?;
    Ok(ast.transpile())
}

pub struct RustSource {
    pub source: Source,
    pub transpiled: Result<String, TranspileError>,
}

impl RustSource {
    fn errors(&self) -> TranspileErrors {
        TranspileErrors {
            source: self.source.clone(),
            errors: self
                .transpiled
                .as_ref()
                .err()
                .map(core::slice::from_ref)
                .unwrap_or_default(),
        }
    }

    fn has_errors(&self) -> bool {
        self.transpiled.is_err()
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

pub fn transpile(source: Source) -> RustSource {
    RustSource {
        source: source.clone(),
        transpiled: transpile_source(source),
    }
}

mod transpile_item {
    mod body;
    mod ident;
    mod r#struct;
    mod task;
    mod toplevel;
    mod r#type;
    mod visibility;
}

trait Transpile {
    fn transpile(self) -> String;
}

trait Punctuated {
    fn punctuation() -> &'static str;
}

mod macros {
    macro_rules! transpile {
        ($string:expr, $($items:expr),*$(,)?) => {
            format!($string, $(($items).transpile()),*)
        };
    }

    macro_rules! impl_transpile {
        ($ty:ty, $string:expr, $($field:ident),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(self) -> String {
                    crate::macros::transpile!($string, $(self.$field),*)
                }
            }
        };
    }

    macro_rules! impl_transpile_fn {
        ($ty:ty, $string:expr, $($fun:ident),*$(,)?) => {
            impl crate::Transpile for $ty {
                fn transpile(self) -> String {
                    crate::macros::transpile!($string, $(self.$fun()),*)
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
                fn transpile(self) -> String {
                    use $ty::*;
                    match self {
                        $($case => crate::macros::transpile!($($args),+),)+
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
                fn transpile(self) -> String {
                    use $ty::*;
                    match self {
                        $($case(inner) => inner.transpile(),)+
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

punct!(", ", TypeElement, TupleTypeMember);
punct!(",\n", StructTypeMember);
punct!("\n\n", RootItem);
// punct!(";\n", Statement);

impl<T> Transpile for Vec<T>
where
    T: Transpile + Punctuated,
{
    fn transpile(self) -> String {
        let punct = T::punctuation();
        self.into_iter()
            .map(|e| e.transpile())
            .reduce(|acc, e| format!("{acc}{punct}{e}"))
            .unwrap_or_else(String::new)
    }
}
