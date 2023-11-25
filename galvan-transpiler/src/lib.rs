pub fn transpile_source(ast: Ast) -> String {
    ast.transpile()
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
    ($string:expr, $($items:expr),*) => {
        format!($string, $(($items).transpile()),*)
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
pub(crate) use transpile;

punct!(", ", TypeItem, TupleTypeMember);
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
