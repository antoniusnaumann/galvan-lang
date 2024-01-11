use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_variants};
use crate::Transpile;
use galvan_ast::{
    ArrayLiteral, CollectionLiteral, DictLiteral, DictLiteralElement, OrderedDictLiteral,
    SetLiteral,
};
use galvan_resolver::Scope;
use itertools::Itertools;

impl_transpile_variants!(CollectionLiteral; ArrayLiteral, DictLiteral, SetLiteral, OrderedDictLiteral);

impl Transpile for ArrayLiteral {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let elements = self
            .elements
            .iter()
            .map(|e| e.transpile(ctx, scope))
            .join(", ");

        format!("vec![{}]", elements)
    }
}

impl_transpile!(DictLiteralElement, "({}, {})", key, value);

impl Transpile for SetLiteral {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let elements = self
            .elements
            .iter()
            .map(|e| e.transpile(ctx, scope))
            .join(", ");

        format!("::std::collections::HashSet::from([{}])", elements)
    }
}

impl_transpile!(
    DictLiteral,
    "::std::collections::HashMap::from([{}])",
    elements
);

impl Transpile for OrderedDictLiteral {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        todo!("Transpile OrderedDictLiteral")
    }
}
