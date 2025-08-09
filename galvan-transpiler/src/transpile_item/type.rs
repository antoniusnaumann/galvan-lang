use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_variants, transpile};
use crate::{Transpile, TypeElement};
use galvan_ast::*;
use galvan_resolver::Scope;

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
impl_transpile!(BasicTypeItem, "{}", ident);
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

impl Transpile for GenericTypeItem {
    fn transpile(&self, _ctx: &Context, _scope: &mut Scope, _errors: &mut crate::ErrorCollector) -> String {
        todo!("Transpile generic type parameters!")
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
    Void
    Infer
    Never
}
