use rustdoc_types::{Crate, Impl, Item, Path, Type};

use galvan_ast::TypeIdent;

/// Fully-qualified path segments for a resolved path.
///
/// Prefers the crate-aware `Crate.paths` entry (the accurate definition module
/// path and owning crate); falls back to the usage string `Path.path` for items
/// that rustdoc only references and does not record in `paths` (e.g. external
/// items). This replaces the old `Path.path` string-splitting heuristics: a bare
/// usage path like `"Option"` now resolves through its `Id` to
/// `["core", "option", "Option"]`, so crate-membership checks are accurate.
fn resolved_path_segments<'a>(krate: &'a Crate, path: &'a Path) -> Vec<&'a str> {
    if let Some(summary) = krate.paths.get(&path.id) {
        summary.path.iter().map(String::as_str).collect()
    } else {
        split_path(&path.path)
    }
}

/// Same as [`resolved_path_segments`], but rewrites a leading `crate`/`$crate`
/// segment to the concrete crate name so generated paths are absolute.
fn resolved_path_segments_with_crate<'a>(
    krate: &'a Crate,
    crate_name: &'a str,
    path: &'a Path,
) -> Vec<&'a str> {
    let mut segments = resolved_path_segments(krate, path);
    if segments
        .first()
        .is_some_and(|segment| matches!(*segment, "crate" | "$crate"))
    {
        segments[0] = crate_name;
    }
    segments
}

fn split_path(path: &str) -> Vec<&str> {
    path.split("::")
        .filter(|segment| !segment.is_empty())
        .collect()
}

pub(super) fn resolved_type_name(path: &Path) -> Option<Box<str>> {
    split_path(&path.path)
        .last()
        .map(|segment| (*segment).into())
}

/// Whether the resolved path is unqualified (no resolvable crate prefix) or its
/// owning crate is one of `crate_names`. Uses the crate-aware
/// [`resolved_path_segments`], so standard-library wrappers are matched by their
/// real crate (`core`/`alloc`/`std`) rather than a fragile usage-string prefix.
pub(super) fn resolved_path_is_unqualified_or_in_crates(
    krate: &Crate,
    path: &Path,
    crate_names: &[&str],
) -> bool {
    let segments = resolved_path_segments(krate, path);
    segments
        .first()
        .is_none_or(|first| crate_names.contains(first))
}

pub(super) fn resolved_path_matches(krate: &Crate, path: &Path, expected: &[&str]) -> bool {
    let actual = resolved_path_segments(krate, path);
    actual.as_slice() == expected || actual.as_slice() == &expected[..expected.len() - 1]
}

pub(super) fn resolved_path_is_unqualified_or_matches_any(
    krate: &Crate,
    path: &Path,
    expected_paths: &[&[&str]],
) -> bool {
    resolved_path_segments(krate, path).is_empty()
        || expected_paths
            .iter()
            .any(|expected| resolved_path_matches(krate, path, expected))
}

pub(super) fn rust_path(krate: &Crate, crate_name: &str, name: &str, item: &Item) -> Box<str> {
    match krate.paths.get(&item.id) {
        Some(summary) if !summary.path.is_empty() => {
            let segments = segments_with_crate(crate_name, &summary.path);
            format!("::{}", segments.join("::")).into()
        }
        _ => format!("::{crate_name}::{name}").into(),
    }
}

pub(super) fn callable_rust_path(
    krate: &Crate,
    crate_name: &str,
    name: &str,
    item: &Item,
) -> Box<str> {
    let path = rust_path(krate, crate_name, name, item);
    if path.ends_with(&format!("::{name}")) {
        path
    } else {
        format!("{path}::{name}").into()
    }
}

pub(super) fn impl_function_rust_path(
    krate: &Crate,
    crate_name: &str,
    name: &str,
    item: &Item,
    impl_: &Impl,
) -> Box<str> {
    let path = callable_rust_path(krate, crate_name, name, item);
    let Some(receiver) = resolved_type_rust_type_path(krate, crate_name, &impl_.for_) else {
        return path;
    };
    if let Some(trait_path) = impl_
        .trait_
        .as_ref()
        .and_then(|trait_| resolved_path_rust_type_path(krate, crate_name, trait_))
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
    krate: &Crate,
    crate_name: &str,
    name: &str,
    item: &Item,
    impl_: &Impl,
) -> Box<str> {
    impl_function_rust_path(krate, crate_name, name, item, impl_)
}

pub(super) fn extension_trait_rust_path(
    krate: &Crate,
    crate_name: &str,
    impl_: &Impl,
    receiver: &TypeIdent,
) -> Option<Box<str>> {
    let trait_ = impl_.trait_.as_ref()?;
    let trait_name = resolved_type_name(trait_)?;
    if trait_name.as_ref() != format!("{}_Ext", receiver.as_str()) {
        return None;
    }
    resolved_path_rust_type_path(krate, crate_name, trait_).map(|path| format!("::{path}").into())
}

pub(super) fn resolved_type_rust_path(
    krate: &Crate,
    crate_name: &str,
    name: &str,
    path: &Path,
) -> Box<str> {
    let segments = resolved_module_segments(krate, crate_name, name, path);
    let full = if segments.is_empty() {
        name.to_string()
    } else {
        format!("{}::{name}", segments.join("::"))
    };

    if full.contains("::") {
        format!("::{full}").into()
    } else {
        format!("::{crate_name}::{full}").into()
    }
}

/// The absolute path to a resolved `Type::ResolvedPath`, e.g. the `for_` type of
/// an impl block.
fn resolved_type_rust_type_path(krate: &Crate, crate_name: &str, ty: &Type) -> Option<Box<str>> {
    let Type::ResolvedPath(path) = ty else {
        return None;
    };
    resolved_path_rust_type_path(krate, crate_name, path)
}

fn resolved_path_rust_type_path(krate: &Crate, crate_name: &str, path: &Path) -> Option<Box<str>> {
    let name = resolved_type_name(path)?;
    let segments = resolved_module_segments(krate, crate_name, name.as_ref(), path);
    if segments.is_empty() {
        Some(name)
    } else {
        Some(format!("{}::{name}", segments.join("::")).into())
    }
}

/// The crate-aware module segments for a resolved path, with the trailing type
/// name removed so callers can re-append it.
fn resolved_module_segments<'a>(
    krate: &'a Crate,
    crate_name: &'a str,
    name: &str,
    path: &'a Path,
) -> Vec<&'a str> {
    let mut segments = resolved_path_segments_with_crate(krate, crate_name, path);
    if segments.last().is_some_and(|segment| *segment == name) {
        segments.pop();
    }
    segments
}

fn segments_with_crate<'a>(crate_name: &'a str, segments: &'a [String]) -> Vec<&'a str> {
    let mut segments: Vec<&str> = segments.iter().map(String::as_str).collect();
    if segments
        .first()
        .is_some_and(|segment| matches!(*segment, "crate" | "$crate"))
    {
        segments[0] = crate_name;
    }
    segments
}
