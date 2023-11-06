use galvan_parser::Ident;

use crate::Transpile;

impl Transpile for Ident {
    fn transpile(self) -> String {
        // TODO: Escape ident when name has collision with rust keyword
        format!("{self}")
    }
}
