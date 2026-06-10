use crate::context::Context;
use crate::error::ErrorCollector;
use crate::sanitize::sanitize_name;
use crate::Transpile;
use galvan_ast::{Ident, TypeIdent};

impl Transpile for Ident {
    fn transpile(&self, _ctx: &Context, _errors: &mut ErrorCollector) -> String {
        // TODO: Use lookup to insert fully qualified name
        sanitize_name(self.as_str()).into()
    }
}

impl Transpile for TypeIdent {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let Some(_decl) = ctx.lookup.types.get(self) else {
            errors.warning(
                format!("Type resolving error: Type {} not found", self),
                None,
            );
            return format!("{self}");
        };
        // TODO: Handle module path here and use fully qualified name
        let name = ctx.mapping.get_owned(self);
        format!("{name}")
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum TypeOwnership {
    Owned,
    MutOwned,
    Borrowed,
    MutBorrowed,
}

pub trait TranspileType {
    fn transpile_type(
        &self,
        ctx: &Context,
        ownership: TypeOwnership,
        errors: &mut ErrorCollector,
    ) -> String;
}

impl TranspileType for TypeIdent {
    fn transpile_type(
        &self,
        ctx: &Context,
        ownership: TypeOwnership,
        errors: &mut ErrorCollector,
    ) -> String {
        let Some(_decl) = ctx.lookup.types.get(self) else {
            errors.warning(
                format!("Type resolving error: Type {} not found", self),
                None,
            );
            let prefix = match ownership {
                TypeOwnership::Owned | TypeOwnership::MutOwned => "",
                TypeOwnership::Borrowed => "&",
                TypeOwnership::MutBorrowed => "&mut",
            };
            return format!("{prefix}{self}");
        };
        // TODO: Handle module path here and use fully qualified name
        let (prefix, name) = match ownership {
            TypeOwnership::Owned | TypeOwnership::MutOwned => ("", ctx.mapping.get_owned(self)),
            TypeOwnership::Borrowed => {
                debug_assert!(
                    !ctx.mapping.is_copy_ident(self),
                    "Tried to borrow a copy type!"
                );
                ("&", ctx.mapping.get_borrowed(self))
            }
            TypeOwnership::MutBorrowed => ("&mut ", ctx.mapping.get_mut_borrowed(self)),
        };
        format!("{prefix}{name}")
    }
}
