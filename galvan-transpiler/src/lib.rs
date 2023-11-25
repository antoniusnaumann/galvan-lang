
pub fn transpile_source(source: Source) -> Result<String, AstError>{
    let ast = source.try_into_ast()?;
    Ok(ast.transpile())
}

mod transpile_item {
    mod body;
    mod ident;
    mod r#struct;
    mod r#type;
    mod task;
    mod toplevel;
    mod visibility;
}

trait Transpile {
    fn transpile(self) -> String;
}


trait Punctuated {
    fn punctuation() -> &'static str;
}

macro_rules! transpile {
    ($string:expr, $($items:expr),*$(,)?) => {
        format!($string, $(($items).transpile()),*)
    };
}

macro_rules! impl_transpile {
    ($ty:ty, $string:expr, $($field:ident),*$(,)?) => {
        impl crate::Transpile for $ty {
            fn transpile(self) -> String {
                crate::transpile!($string, $(self.$field),*)
            }
        }
    };
}

macro_rules! impl_transpile_fn {
    ($ty:ty, $string:expr, $($fun:ident),*$(,)?) => {
        impl crate::Transpile for $ty {
            fn transpile(self) -> String {
                crate::transpile!($string, $(self.$fun()),*)
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
                    $($case => crate::transpile!($($args),+),)+
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

use galvan_ast::*;
pub(crate) use {transpile, impl_transpile_match, impl_transpile, impl_transpile_fn, impl_transpile_variants};

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
