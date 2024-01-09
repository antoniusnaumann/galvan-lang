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
pub enum TypeOwnership {
    Owned,
    MutOwned,
    Borrowed,
    MutBorrowed,
}

pub trait TranspileType {
    fn transpile_type(&self, ctx: &Context, scope: &mut Scope, ownership: TypeOwnership) -> String;
}

impl TranspileType for TypeIdent {
    fn transpile_type(&self, ctx: &Context, scope: &mut Scope, ownership: TypeOwnership) -> String {
        let Some(decl) = ctx.lookup.types.get(self) else {
            todo!("Handle type resolving errors. Type {} not found", self);
        };
        // TODO: Handle module path here and use fully qualified name
        let name = match ownership {
            TypeOwnership::Owned => ctx.mapping.get_owned(self),
            TypeOwnership::MutOwned => todo!("Transpile mutable owned types"), // ctx.mapping.get_mut_owned(&self),
            TypeOwnership::Borrowed => ctx.mapping.get_borrowed(self),
            TypeOwnership::MutBorrowed => ctx.mapping.get_mut_borrowed(self),
        };
        format!("{name}")
    }
}
