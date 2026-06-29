use std::collections::HashSet;

use serde_json::Value;

use galvan_ast::{TypeElement, TypeIdent};

pub(super) fn inner<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(object) => object.get(key),
        _ => None,
    }
}

pub(super) fn inner_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    inner(value, key).and_then(Value::as_str)
}

pub(super) fn item_inner<'a>(item: &'a Value, key: &str) -> Option<&'a Value> {
    item.get("inner").and_then(|inner| inner.get(key))
}

pub(super) fn is_public(item: &Value) -> bool {
    item.get("visibility")
        .is_some_and(|visibility| match visibility {
            Value::String(value) => value == "public",
            Value::Object(object) => object.contains_key("public"),
            _ => false,
        })
}

pub(super) fn public_type_name(item: &Value) -> Option<&str> {
    let name = item.get("name").and_then(Value::as_str)?;
    let inner = item.get("inner")?;
    ["struct", "enum", "type_alias", "union"]
        .iter()
        .any(|kind| inner.get(*kind).is_some())
        .then_some(name)
}

pub(super) fn item_ids<'a>(item: &'a Value, key: &str) -> Vec<&'a str> {
    item.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

pub(super) fn impl_function_ids(index: &serde_json::Map<String, Value>) -> HashSet<&str> {
    index
        .values()
        .filter_map(|item| item_inner(item, "impl"))
        .flat_map(|impl_item| item_ids(impl_item, "items"))
        .filter(|id| index.get(*id).and_then(item_inner_constant).is_none())
        .collect()
}

pub(super) fn impl_constant_ids(index: &serde_json::Map<String, Value>) -> HashSet<&str> {
    index
        .values()
        .filter_map(|item| item_inner(item, "impl"))
        .flat_map(|impl_item| item_ids(impl_item, "items"))
        .filter(|id| index.get(*id).and_then(item_inner_constant).is_some())
        .collect()
}

pub(super) fn rust_path(crate_name: &str, name: &str, item: &Value) -> Box<str> {
    item.get("path")
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("::")
        })
        .filter(|path| !path.is_empty())
        .map(|path| format!("::{path}").into())
        .unwrap_or_else(|| format!("::{crate_name}::{name}").into())
}

pub(super) fn callable_rust_path(crate_name: &str, name: &str, item: &Value) -> Box<str> {
    let path = rust_path(crate_name, name, item);
    if path.ends_with(&format!("::{name}")) {
        path
    } else {
        format!("{path}::{name}").into()
    }
}

pub(super) fn impl_function_rust_path(
    crate_name: &str,
    name: &str,
    item: &Value,
    impl_inner: &Value,
) -> Box<str> {
    let path = callable_rust_path(crate_name, name, item);
    let Some(receiver) = impl_inner.get("for").and_then(resolved_rust_type_path) else {
        return path;
    };
    if let Some(trait_path) = impl_inner
        .get("trait")
        .filter(|trait_| !trait_.is_null())
        .and_then(resolved_rust_type_path)
    {
        return format!("<::{receiver} as ::{trait_path}>::{name}").into();
    }
    if path.matches("::").count() > 2 {
        return path;
    };
    if receiver.contains("::") {
        format!("::{receiver}::{name}").into()
    } else {
        format!("::{crate_name}::{receiver}::{name}").into()
    }
}

pub(super) fn impl_constant_rust_path(
    crate_name: &str,
    name: &str,
    item: &Value,
    impl_inner: &Value,
) -> Box<str> {
    impl_function_rust_path(crate_name, name, item, impl_inner)
}

fn resolved_rust_type_path(ty: &Value) -> Option<Box<str>> {
    let resolved = inner(ty, "resolved_path")?;
    let name = resolved.get("name").and_then(Value::as_str)?;
    let mut segments = resolved
        .get("path")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if segments.last().is_some_and(|segment| *segment == name) {
        segments.pop();
    }

    if segments.is_empty() {
        Some(name.into())
    } else {
        Some(format!("{}::{name}", segments.join("::")).into())
    }
}

fn item_inner_constant(item: &Value) -> Option<&Value> {
    item_inner(item, "constant").or_else(|| item_inner(item, "assoc_const"))
}

pub(super) fn constant_inner(item: &Value) -> Option<&Value> {
    item_inner_constant(item)
}

pub(super) fn constant_type(constant: &Value) -> Option<&Value> {
    constant.get("type").or_else(|| constant.get("ty"))
}

pub(super) fn receiver_type_ident(ty: &TypeElement) -> Option<TypeIdent> {
    match ty {
        TypeElement::Plain(plain) => Some(plain.ident.clone()),
        TypeElement::Parametric(parametric) => Some(parametric.base_type.clone()),
        TypeElement::Generic(generic) => Some(TypeIdent::new(generic.ident.as_str())),
        _ => None,
    }
}

pub(super) fn return_is_borrowed(signature: &Value) -> bool {
    signature
        .get("output")
        .is_some_and(|output| inner(output, "borrowed_ref").is_some())
}

pub(super) fn type_is_owned(ty: &Value) -> bool {
    inner(ty, "borrowed_ref").is_none()
}

pub(super) fn borrowed_ref_is_mutable(borrowed: &Value) -> bool {
    borrowed.get("mutable").and_then(Value::as_bool) == Some(true)
        || borrowed.get("mutability").and_then(Value::as_str) == Some("mut")
}

pub(super) fn resolved_type_args(resolved: &Value) -> Vec<&Value> {
    resolved
        .get("args")
        .and_then(|args| inner(args, "angle_bracketed"))
        .and_then(|args| args.get("args"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|arg| inner(arg, "type"))
        .collect()
}
