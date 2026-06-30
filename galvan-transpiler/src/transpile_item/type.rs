use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_variants, transpile};
use crate::{ErrorCollector, Transpile};
use galvan_ast::*;
use galvan_resolver::Lookup;
use itertools::Itertools;

// TODO: Re-export used types from galvan library to avoid referencing the used crates directly

impl_transpile!(ArrayTypeItem, "::std::vec::Vec<{}>", elements);
impl_transpile!(
    DictionaryTypeItem,
    "::std::collections::HashMap<{}, {}>",
    key,
    value
);
impl_transpile!(
    OrderedDictionaryTypeItem,
    "::std::collections::BTreeMap<{}, {}>",
    key,
    value
);
impl_transpile!(SetTypeItem, "::std::collections::HashSet<{}>", elements);
impl_transpile!(TupleTypeItem, "({})", elements);
impl_transpile!(OptionalTypeItem, "Option<{}>", inner);

impl Transpile for BasicTypeItem {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let base_name = self.ident.transpile(ctx, errors);

        // Check if this is a known type with generic parameters in the current context
        // We need to look up the type definition and see if it has generics
        if let Some(type_item) = ctx.lookup.resolve_type(&self.ident) {
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
    fn transpile(&self, ctx: &Context, errors: &mut crate::ErrorCollector) -> String {
        let ResultTypeItem {
            success,
            error,
            span: _span,
        } = self;
        if let Some(error) = error {
            transpile!(ctx, errors, "Result<{}, {}>", success, error)
        } else {
            transpile!(ctx, errors, "::galvan::std::FlexResult<{}>", success)
        }
    }
}

impl Transpile for ParametricTypeItem {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        if let Some(pointer) = raw_pointer_prefix(self.base_type.as_str()) {
            let Some(inner) = self.type_args.first() else {
                errors.warning(
                    format!(
                        "Raw pointer type {} is missing its pointee type",
                        self.base_type
                    ),
                    None,
                );
                return format!("{pointer} _");
            };
            return format!("{pointer} {}", inner.transpile(ctx, errors));
        }

        let base = self.base_type.transpile(ctx, errors);
        let args = self
            .type_args
            .iter()
            .map(|arg| arg.transpile(ctx, errors))
            .join(", ");
        format!("{}<{}>", base, args)
    }
}

fn raw_pointer_prefix(base_type: &str) -> Option<&'static str> {
    match base_type {
        "ConstRawPointer" => Some("*const"),
        "MutRawPointer" => Some("*mut"),
        _ => None,
    }
}

impl Transpile for GenericTypeItem {
    fn transpile(&self, _ctx: &Context, _errors: &mut ErrorCollector) -> String {
        // Generic type parameters should be capitalized for Rust conventions
        crate::capitalize_generic(self.ident.as_str())
    }
}

impl Transpile for ClosureTypeItem {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let params = self
            .parameters
            .iter()
            .map(|p| {
                let ty = p.transpile(ctx, errors);
                if ctx.mapping.is_copy(p) {
                    ty
                } else {
                    format!("&{ty}")
                }
            })
            .join(", ");

        // TODO: We should somehow give users a way to declare that an Fn instead of an FnMut is desired here, e.g., for multithreading
        transpile!(ctx, errors, "impl Fn({params}) -> {}", self.return_ty)
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
    Closure
    Void
    Infer
    Never
}

#[cfg(test)]
mod tests {
    use galvan_ast::{BasicTypeItem, ParametricTypeItem, Span, TypeElement, TypeIdent};
    use galvan_hir::mapping::Mapping;

    use crate::context::Context;
    use crate::{ErrorCollector, Transpile};

    #[test]
    fn synthetic_raw_pointer_types_transpile_to_rust_pointers() {
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();
        let const_pointer = ParametricTypeItem {
            base_type: TypeIdent::new("ConstRawPointer"),
            type_args: vec![TypeElement::Plain(BasicTypeItem {
                ident: TypeIdent::new("Ticket"),
                span: Span::default(),
            })],
            span: Span::default(),
        };
        let mut_pointer = ParametricTypeItem {
            base_type: TypeIdent::new("MutRawPointer"),
            type_args: vec![TypeElement::Plain(BasicTypeItem {
                ident: TypeIdent::new("String"),
                span: Span::default(),
            })],
            span: Span::default(),
        };

        assert_eq!(const_pointer.transpile(&ctx, &mut errors), "*const Ticket");
        assert_eq!(mut_pointer.transpile(&ctx, &mut errors), "*mut String");
    }
}
