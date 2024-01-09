use crate::context::Context;
use crate::sanitize::sanitize_name;
use crate::{Ident, Transpile, TypeIdent};
use galvan_resolver::Scope;

impl Transpile for Ident {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        // TODO: Escape ident when name has collision with rust keyword
        // TODO: Use lookup to insert fully qualified name
        sanitize_name(self.as_str()).into()
    }
}

impl Transpile for TypeIdent {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Some(decl) = ctx.lookup.types.get(self) else {
            todo!("Handle type resolving errors. Type {} not found", self);
        };
        // TODO: Handle module path here and use fully qualified name
        let name = ctx.mapping.get_owned(self);
        format!("{name}")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Ownership {
    Owned,
    MutOwned,
    Borrowed,
    MutBorrowed,
}

pub trait TranspileType {
    fn transpile_type(&self, ctx: &Context, scope: &mut Scope, ownership: Ownership) -> String;
}

impl TranspileType for TypeIdent {
    fn transpile_type(&self, ctx: &Context, scope: &mut Scope, ownership: Ownership) -> String {
        let Some(decl) = ctx.lookup.types.get(self) else {
            todo!("Handle type resolving errors. Type {} not found", self);
        };
        // TODO: Handle module path here and use fully qualified name
        let name = match ownership {
            Ownership::Owned => ctx.mapping.get_owned(self),
            Ownership::MutOwned => todo!("Transpile mutable owned types"), // ctx.mapping.get_mut_owned(&self),
            Ownership::Borrowed => ctx.mapping.get_borrowed(self),
            Ownership::MutBorrowed => ctx.mapping.get_mut_borrowed(self),
        };
        format!("{name}")
    }
}
