use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_variants, transpile};
use crate::{ErrorCollector, Transpile, TypeElement};
use galvan_ast::*;
use galvan_resolver::{Lookup, Scope};
use itertools::Itertools;

// TODO: Re-export used types from galvan library to avoid referencing the used crates directly

impl_transpile!(ArrayTypeItem, "::std::vec::Vec<{}>", elements);
impl_transpile!(
    DictionaryTypeItem,
    "::std::collections::HashMap<{}, {}>",
    key,
    value
);
impl_transpile!(OrderedDictionaryTypeItem, "TODO {} {}", key, value);
impl_transpile!(SetTypeItem, "::std::collections::HashSet<{}>", elements);
impl_transpile!(TupleTypeItem, "({})", elements);
impl_transpile!(OptionalTypeItem, "Option<{}>", inner);
impl Transpile for BasicTypeItem {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let base_name = self.ident.transpile(ctx, scope, errors);
        
        // Check if this is a known type with generic parameters in the current context
        // We need to look up the type definition and see if it has generics
        if let Some(type_item) = scope.resolve_type(&self.ident) {
            let generics = type_item.item.collect_generics();
            if !generics.is_empty() {
                // This is a generic type, add <_> for each generic parameter
                let placeholders = vec!["_"; generics.len()].join(", ");
                return format!("{}<{}>", base_name, placeholders);
            }
        }
        
        // Not a generic type or can't determine, use the base name
        base_name
    }
}
impl_transpile!(VoidTypeItem, "()",);
impl_transpile!(InferTypeItem, "_",);
impl_transpile!(NeverTypeItem, "!",);

impl Transpile for ResultTypeItem {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut crate::ErrorCollector) -> String {
        let ResultTypeItem {
            success,
            error,
            span: _span,
        } = self;
        if let Some(error) = error {
            transpile!(ctx, scope, errors, "Result<{}, {}>", success, error)
        } else {
            transpile!(ctx, scope, errors, "::galvan::std::FlexResult<{}>", success)
        }
    }
}

impl Transpile for ParametricTypeItem {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let base = self.base_type.transpile(ctx, scope, errors);
        let args = self.type_args
            .iter()
            .map(|arg| arg.transpile(ctx, scope, errors))
            .join(", ");
        format!("{}<{}>", base, args)
    }
}

impl Transpile for GenericTypeItem {
    fn transpile(&self, _ctx: &Context, _scope: &mut Scope, _errors: &mut ErrorCollector) -> String {
        // Generic type parameters should be capitalized for Rust conventions
        crate::capitalize_generic(self.ident.as_str())
    }
}

impl_transpile_variants! { TypeElement;
    Plain
    Array
    Dictionary
    OrderedDictionary
    Set
    Tuple
    Optional
    Result
    Generic
    Parametric
    Void
    Infer
    Never
}
