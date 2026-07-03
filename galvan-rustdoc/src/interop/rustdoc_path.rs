use serde_json::Value;

use super::rustdoc_json::inner;

pub(super) fn rust_path(crate_name: &str, name: &str, item: &Value) -> Box<str> {
    item.get("path")
        .and_then(|path| path_segments_with_crate(crate_name, path))
        .map(|segments| segments.join("::"))
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
    let Some(receiver) = impl_inner
        .get("for")
        .and_then(|ty| resolved_rust_type_path(crate_name, ty))
    else {
        return path;
    };
    if let Some(trait_path) = impl_inner
        .get("trait")
        .filter(|trait_| !trait_.is_null())
        .and_then(|ty| resolved_rust_type_path(crate_name, ty))
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

pub(super) fn resolved_type_rust_path(crate_name: &str, name: &str, resolved: &Value) -> Box<str> {
    let path = resolved_path_segments(crate_name, name, resolved)
        .map(|segments| {
            if segments.is_empty() {
                name.to_string()
            } else {
                format!("{}::{name}", segments.join("::"))
            }
        })
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| name.to_string());

    if path.contains("::") {
        format!("::{path}").into()
    } else {
        format!("::{crate_name}::{path}").into()
    }
}

pub(super) fn resolved_type_name(resolved: &Value) -> Option<Box<str>> {
    if let Some(name) = resolved.get("name").and_then(Value::as_str) {
        return Some(name.into());
    }

    resolved_path_segments_raw(resolved)
        .and_then(|segments| segments.last().map(|segment| (*segment).into()))
}

pub(super) fn resolved_path_segments_raw(resolved: &Value) -> Option<Vec<&str>> {
    resolved.get("path").and_then(path_segments)
}

fn resolved_rust_type_path(crate_name: &str, ty: &Value) -> Option<Box<str>> {
    let resolved = inner(ty, "resolved_path")?;
    let name = resolved_type_name(resolved)?;
    let segments = resolved_path_segments(crate_name, name.as_ref(), resolved)?;

    if segments.is_empty() {
        Some(name)
    } else {
        Some(format!("{}::{name}", segments.join("::")).into())
    }
}

fn resolved_path_segments<'a>(
    crate_name: &'a str,
    name: &str,
    resolved: &'a Value,
) -> Option<Vec<&'a str>> {
    let mut segments = resolved
        .get("path")
        .and_then(|path| path_segments_with_crate(crate_name, path))?;
    if segments.last().is_some_and(|segment| *segment == name) {
        segments.pop();
    }
    Some(segments)
}

fn path_segments_with_crate<'a>(crate_name: &'a str, path: &'a Value) -> Option<Vec<&'a str>> {
    let mut segments = path_segments(path)?;
    if segments
        .first()
        .is_some_and(|segment| matches!(*segment, "crate" | "$crate"))
    {
        segments[0] = crate_name;
    }
    Some(segments)
}

fn path_segments(path: &Value) -> Option<Vec<&str>> {
    match path {
        Value::Array(_) => Some(
            path.as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|segment| !segment.is_empty())
                .collect(),
        ),
        Value::String(path) => Some(
            path.split("::")
                .filter(|segment| !segment.is_empty())
                .collect(),
        ),
        _ => None,
    }
}
