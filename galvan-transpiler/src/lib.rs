mod transpile_item {
    mod ident;
    mod r#struct;
    mod r#type;
    mod visibility;
}

macro_rules! transpile {
    ($string:expr, $($items:expr),*) => {
        format!($string, $(($items).transpile()),*)
    };
}

use galvan_parser::{ParsedSource, RootItem};
pub(crate) use transpile;

pub fn transpile_source(parsed_source: ParsedSource) -> String {
    parsed_source
        .into_iter()
        .map(|e| e.transpile())
        .reduce(|acc, e| format!("{acc}\n{e}"))
        .unwrap_or_else(String::new)
}

trait Transpile {
    fn transpile(self) -> String;
}

impl Transpile for RootItem {
    fn transpile(self) -> String {
        match self {
            RootItem::Fn(_) => todo!(),
            RootItem::Type(t) => transpile!("{}", t),
            RootItem::Main(_) => todo!(),
            RootItem::Test(_) => todo!(),
            RootItem::CustomTask(_) => todo!(),
        }
    }
}

impl<T> Transpile for Vec<T>
where
    T: Transpile,
{
    fn transpile(self) -> String {
        self.into_iter()
            .map(|e| e.transpile())
            .reduce(|acc, e| format!("{acc}, {e}"))
            .unwrap_or_else(String::new)
    }
}
