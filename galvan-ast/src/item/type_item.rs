use std::fmt;

use galvan_ast_macro::AstNode;
use typeunion::type_union;

use crate::{AstNode, Ident, PrintAst, Span, TypeIdent};

type Array = Box<ArrayTypeItem>;
type Dictionary = Box<DictionaryTypeItem>;
type OrderedDictionary = Box<OrderedDictionaryTypeItem>;
type Set = Box<SetTypeItem>;
type Tuple = Box<TupleTypeItem>;
type Optional = Box<OptionalTypeItem>;
type Result = Box<ResultTypeItem>;
type Plain = BasicTypeItem;
type Generic = GenericTypeItem;
type Parametric = ParametricTypeItem;
type Void = VoidTypeItem;
type Infer = InferTypeItem;
type Never = NeverTypeItem;

#[type_union]
#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub type TypeElement = Array
    + Dictionary
    + OrderedDictionary
    + Set
    + Tuple
    + Optional
    + Result
    + Plain
    + Generic
    + Parametric
    + Infer
    + Void
    + Never;

impl TypeElement {
    pub fn bool() -> Self {
        BasicTypeItem {
            ident: TypeIdent::new("Bool"),
            span: Span::default(),
        }
        .into()
    }

    pub fn infer() -> Self {
        InferTypeItem::default().into()
    }

    pub fn void() -> Self {
        VoidTypeItem::default().into()
    }

    pub fn collect_generics_recursive(&self, generics: &mut std::collections::HashSet<Ident>) {
        self.collect_generics_recursive_with_depth(generics, 0, 32);
    }

    fn collect_generics_recursive_with_depth(&self, generics: &mut std::collections::HashSet<Ident>, depth: u32, max_depth: u32) {
        if depth >= max_depth {
            // Prevent infinite recursion
            return;
        }
        
        match self {
            TypeElement::Generic(gen) => {
                generics.insert(gen.ident.clone());
            }
            TypeElement::Parametric(param) => {
                // Only collect from type arguments, not the base type itself
                for arg in &param.type_args {
                    arg.collect_generics_recursive_with_depth(generics, depth + 1, max_depth);
                }
            }
            TypeElement::Array(arr) => arr.elements.collect_generics_recursive_with_depth(generics, depth + 1, max_depth),
            TypeElement::Dictionary(dict) => {
                dict.key.collect_generics_recursive_with_depth(generics, depth + 1, max_depth);
                dict.value.collect_generics_recursive_with_depth(generics, depth + 1, max_depth);
            }
            TypeElement::OrderedDictionary(dict) => {
                dict.key.collect_generics_recursive_with_depth(generics, depth + 1, max_depth);
                dict.value.collect_generics_recursive_with_depth(generics, depth + 1, max_depth);
            }
            TypeElement::Set(set) => set.elements.collect_generics_recursive_with_depth(generics, depth + 1, max_depth),
            TypeElement::Tuple(tuple) => {
                for elem in &tuple.elements {
                    elem.collect_generics_recursive_with_depth(generics, depth + 1, max_depth);
                }
            }
            TypeElement::Optional(opt) => opt.inner.collect_generics_recursive_with_depth(generics, depth + 1, max_depth),
            TypeElement::Result(res) => {
                res.success.collect_generics_recursive_with_depth(generics, depth + 1, max_depth);
                if let Some(error) = &res.error {
                    error.collect_generics_recursive_with_depth(generics, depth + 1, max_depth);
                }
            }
            // No generics in these cases
            TypeElement::Plain(_)
            | TypeElement::Void(_)
            | TypeElement::Infer(_)
            | TypeElement::Never(_) => {}
        }
    }
}

impl Default for TypeElement {
    fn default() -> Self {
        Self::infer()
    }
}

// TODO: Add a marker trait to constrain this to only type decls
#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct ArrayTypeItem {
    pub elements: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct DictionaryTypeItem {
    pub key: TypeElement,
    pub value: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct OrderedDictionaryTypeItem {
    pub key: TypeElement,
    pub value: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct SetTypeItem {
    pub elements: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct TupleTypeItem {
    pub elements: Vec<TypeElement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct OptionalTypeItem {
    pub inner: TypeElement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct ResultTypeItem {
    pub success: TypeElement,
    pub error: Option<TypeElement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct BasicTypeItem {
    pub ident: TypeIdent,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct GenericTypeItem {
    pub ident: Ident,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, AstNode)]
pub struct ParametricTypeItem {
    pub base_type: TypeIdent,
    pub type_args: Vec<TypeElement>,
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, AstNode)]
pub struct NeverTypeItem {
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, AstNode)]
pub struct VoidTypeItem {
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, AstNode)]
pub struct InferTypeItem {
    pub span: Span,
}

impl fmt::Display for TypeElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeElement::Array(arr) => write!(f, "[{}]", arr.elements),
            TypeElement::Dictionary(dict) => write!(f, "[{}: {}]", dict.key, dict.value),
            TypeElement::OrderedDictionary(dict) => write!(f, "[{}: {}]", dict.key, dict.value),
            TypeElement::Set(set) => write!(f, "{{{}}}", set.elements),
            TypeElement::Tuple(tuple) => {
                write!(f, "(")?;
                for (i, elem) in tuple.elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
            TypeElement::Optional(opt) => write!(f, "{}?", opt.inner),
            TypeElement::Result(res) => match &res.error {
                Some(err) => write!(f, "Result<{}, {}>", res.success, err),
                None => write!(f, "Result<{}>", res.success),
            },
            TypeElement::Plain(basic) => write!(f, "{}", basic.ident),
            TypeElement::Generic(gen) => write!(f, "{}", gen.ident),
            TypeElement::Parametric(param) => {
                write!(f, "{}<", param.base_type)?;
                for (i, arg) in param.type_args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            TypeElement::Void(_) => write!(f, "Void"),
            TypeElement::Infer(_) => write!(f, "_"),
            TypeElement::Never(_) => write!(f, "!"),
        }
    }
}
