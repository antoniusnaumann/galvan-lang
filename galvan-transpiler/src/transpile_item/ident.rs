use crate::{Ident, TypeIdent, Transpile};

impl Transpile for Ident {
    fn transpile(self) -> String {
        // TODO: Escape ident when name has collision with rust keyword
        format!("{self}")
    }
}

impl Transpile for TypeIdent {
    fn transpile(self) -> String {
        format!("{self}")
    }
}