mod transpile_struct;
mod transpile_type;

macro_rules! transpile {
    ($string:expr, $($items:expr),*) => {
        format!($string, $(($items).transpile()),*)
    };
}

pub(crate) use transpile;

trait Transpile {
    fn transpile(self) -> String;
}

impl<T> Transpile for Vec<T>
where
    T: Transpile,
{
    fn transpile(self) -> String {
        self.into_iter()
            .map(|e| e.transpile())
            .fold("".to_string(), |acc, e| format!("{acc}, {e}"))
    }
}
