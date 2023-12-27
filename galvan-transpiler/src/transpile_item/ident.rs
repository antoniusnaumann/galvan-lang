use crate::{Ident, LookupContext, Transpile, TypeIdent};

impl Transpile for Ident {
    fn transpile(&self, lookup: &LookupContext) -> String {
        // TODO: Escape ident when name has collision with rust keyword
        // TODO: Use lookup to insert fully qualified name
        format!("{self}")
    }
}

impl Transpile for TypeIdent {
    fn transpile(&self, lookup: &LookupContext) -> String {
        let Some(decl) = lookup.types.get(&self.into()) else {
            todo!("Handle type resolving errors")
        };
        // TODO: Handle module path here and use fully qualified name
        let name = decl.extern_name();
        format!("{name}")
    }
}
