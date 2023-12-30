use crate::context::Context;
use crate::{Ident, Transpile, TypeIdent};

impl Transpile for Ident {
    fn transpile(&self, ctx: &Context) -> String {
        // TODO: Escape ident when name has collision with rust keyword
        // TODO: Use lookup to insert fully qualified name
        format!("{self}")
    }
}

impl Transpile for TypeIdent {
    fn transpile(&self, ctx: &Context) -> String {
        let Some(decl) = ctx.lookup.types.get(&self.into()) else {
            todo!("Handle type resolving errors. Type {} not found", self);
        };
        // TODO: Handle module path here and use fully qualified name
        let name = ctx.mapping.get_owned(self);
        format!("{name}")
    }
}
