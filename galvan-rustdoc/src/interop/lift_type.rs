use serde_json::Value;

use galvan_ast::{
    ArrayTypeItem, BasicTypeItem, Ident, ParametricTypeItem, ResultTypeItem, Span, TypeElement,
    TypeIdent,
};

use crate::model::{RustArgConversion, RustReturnConversion};

use super::lift_model::LiftedType;
use super::rustdoc_path::resolved_path_segments_raw;

pub(super) fn result_type(success: Option<&LiftedType>, error: Option<TypeElement>) -> LiftedType {
    LiftedType::new(TypeElement::Result(Box::new(ResultTypeItem {
        success: success
            .map(|arg| arg.ty.clone())
            .unwrap_or_else(TypeElement::infer),
        error,
        span: Span::default(),
    })))
}

pub(super) fn member_arg_conversion(return_conversion: RustReturnConversion) -> RustArgConversion {
    match return_conversion {
        RustReturnConversion::None => RustArgConversion::None,
        RustReturnConversion::BoxDeref => RustArgConversion::BoxNew,
        RustReturnConversion::RcCloneDeref => RustArgConversion::RcNew,
    }
}

pub(super) fn parametric_or_plain_type(name: &str, args: Vec<LiftedType>) -> TypeElement {
    if args.is_empty() {
        return plain_type(TypeIdent::new(name));
    }

    TypeElement::Parametric(ParametricTypeItem {
        base_type: TypeIdent::new(name),
        type_args: args.into_iter().map(|arg| arg.ty).collect(),
        span: Span::default(),
    })
}

pub(super) fn array_type(inner: LiftedType) -> LiftedType {
    LiftedType::new(TypeElement::Array(Box::new(ArrayTypeItem {
        elements: inner.ty,
        span: Span::default(),
    })))
}

pub(super) fn function_pointer_input_type(input: &Value) -> &Value {
    input
        .as_array()
        .and_then(|pair| pair.get(1))
        .unwrap_or(input)
}

pub(super) fn resolved_path_matches(resolved: &Value, expected: &[&str]) -> bool {
    let Some(actual) = resolved_path_segments_raw(resolved) else {
        return false;
    };
    actual.as_slice() == expected || actual.as_slice() == &expected[..expected.len() - 1]
}

pub(super) fn atomic_type(name: &str) -> Option<TypeElement> {
    let galvan = match name {
        "AtomicBool" => "Bool",
        "AtomicI8" => "I8",
        "AtomicI16" => "I16",
        "AtomicI32" => "I32",
        "AtomicI64" => "I64",
        "AtomicIsize" => "ISize",
        "AtomicU8" => "U8",
        "AtomicU16" => "U16",
        "AtomicU32" => "U32",
        "AtomicU64" => "U64",
        "AtomicUsize" => "USize",
        _ => return None,
    };
    Some(plain_type(TypeIdent::new(galvan)))
}

pub(super) fn string_type() -> TypeElement {
    plain_type(TypeIdent::new("String"))
}

pub(super) fn plain_type(ident: TypeIdent) -> TypeElement {
    TypeElement::Plain(BasicTypeItem {
        ident,
        span: Span::default(),
    })
}

pub(super) fn generic_type(name: &str) -> TypeElement {
    TypeElement::Generic(galvan_ast::GenericTypeItem {
        ident: Ident::new(name),
        span: Span::default(),
    })
}

pub(super) fn primitive_type(name: &str) -> TypeElement {
    let galvan = match name {
        "!" => return never_type(),
        "bool" => "Bool",
        "i8" => "I8",
        "i16" => "I16",
        "i32" => "I32",
        "i64" => "I64",
        "i128" => "I128",
        "isize" => "ISize",
        "u8" => "U8",
        "u16" => "U16",
        "u32" => "U32",
        "u64" => "U64",
        "u128" => "U128",
        "usize" => "USize",
        "f32" => "Float",
        "f64" => "Double",
        "char" => "Char",
        "str" => return string_type(),
        _ => "__UnknownRustPrimitive",
    };
    plain_type(TypeIdent::new(galvan))
}

pub(super) fn type_is_copy(ty: &TypeElement) -> bool {
    match ty {
        TypeElement::Plain(plain) => plain_type_is_copy(plain.ident.as_str()),
        TypeElement::Tuple(tuple) => tuple.elements.iter().all(type_is_copy),
        TypeElement::Optional(optional) => type_is_copy(&optional.inner),
        TypeElement::Result(result) => {
            type_is_copy(&result.success) && result.error.as_ref().is_some_and(type_is_copy)
        }
        TypeElement::Void(_) => true,
        TypeElement::Array(_)
        | TypeElement::Dictionary(_)
        | TypeElement::OrderedDictionary(_)
        | TypeElement::Set(_)
        | TypeElement::Generic(_)
        | TypeElement::Parametric(_)
        | TypeElement::Closure(_)
        | TypeElement::Infer(_)
        | TypeElement::Never(_) => false,
    }
}

pub(super) fn never_type() -> TypeElement {
    TypeElement::Never(galvan_ast::NeverTypeItem {
        span: Span::default(),
    })
}

fn plain_type_is_copy(name: &str) -> bool {
    matches!(
        name,
        "Bool"
            | "I8"
            | "I16"
            | "I32"
            | "I64"
            | "I128"
            | "ISize"
            | "U8"
            | "U16"
            | "U32"
            | "U64"
            | "U128"
            | "USize"
            | "Float"
            | "Double"
            | "Char"
    )
}
