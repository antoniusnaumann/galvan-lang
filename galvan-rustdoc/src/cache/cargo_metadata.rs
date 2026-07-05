use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

use crate::RustdocError;

/// A dependency located in the current workspace's cargo metadata.
pub(super) struct ResolvedDependency {
    /// Path to the dependency's `Cargo.toml`, used to run rustdoc.
    pub(super) manifest_path: PathBuf,
    /// The dependency's library target name. rustdoc names its JSON output
    /// after this name, which can differ from both the package name (dashes
    /// are normalized to underscores) and the Galvan crate ident (an explicit
    /// `[lib] name` overrides the default).
    pub(super) lib_name: String,
}

/// Target kinds that rustdoc can document as a library. A package has at most
/// one such target; everything else (`bin`, `example`, `test`, ...) is ignored.
const LIBRARY_KINDS: &[&str] = &[
    "lib",
    "rlib",
    "dylib",
    "cdylib",
    "staticlib",
    "proc-macro",
];

pub(super) fn dependency_manifest_path(
    crate_name: &str,
) -> Result<Option<ResolvedDependency>, RustdocError> {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let manifest_path = manifest_dir.join("Cargo.toml");
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest_path)
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .env_remove("RUSTC_WRAPPER")
        .output()
        .map_err(RustdocError::CargoMetadata)?;

    let metadata: Value =
        serde_json::from_slice(&output.stdout).map_err(RustdocError::InvalidCargoMetadata)?;

    Ok(resolve_dependency(&metadata, crate_name))
}

/// Locates a dependency in `cargo metadata` output by matching the Galvan crate
/// ident against the package's normalized name.
///
/// Galvan namespaces are idents, so `crate_name` always uses underscores. Cargo
/// package names may contain dashes (`some-crate`), so the comparison normalizes
/// dashes to underscores before matching.
fn resolve_dependency(metadata: &Value, crate_name: &str) -> Option<ResolvedDependency> {
    let package = metadata
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|package| {
            package
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| normalize_crate_name(name) == crate_name)
        })?;

    let manifest_path = package
        .get("manifest_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)?;

    // rustdoc names its JSON after the library target. Fall back to the
    // normalized package name when no library target is reported (in which case
    // `cargo rustdoc --lib` will fail loudly downstream anyway).
    let lib_name = library_target_name(package).unwrap_or_else(|| crate_name.to_string());

    Some(ResolvedDependency {
        manifest_path,
        lib_name,
    })
}

/// Returns the name of the package's library target, if it has one.
fn library_target_name(package: &Value) -> Option<String> {
    package
        .get("targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|target| {
            target
                .get("kind")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .any(|kind| LIBRARY_KINDS.contains(&kind))
        })
        .and_then(|target| target.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Normalizes a cargo package name to a crate ident by replacing dashes with
/// underscores, matching how Rust (and Galvan namespaces) refer to the crate.
fn normalize_crate_name(name: &str) -> String {
    name.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn matches_dashed_package_by_underscored_ident() {
        let metadata = json!({
            "packages": [{
                "name": "some-crate",
                "manifest_path": "/deps/some-crate/Cargo.toml",
                "targets": [{ "name": "some_crate", "kind": ["lib"] }],
            }],
        });

        let resolved = resolve_dependency(&metadata, "some_crate").expect("dependency resolved");
        assert_eq!(
            resolved.manifest_path,
            PathBuf::from("/deps/some-crate/Cargo.toml")
        );
        assert_eq!(resolved.lib_name, "some_crate");
    }

    #[test]
    fn prefers_explicit_lib_name_over_package_name() {
        let metadata = json!({
            "packages": [{
                "name": "some-crate",
                "manifest_path": "/deps/some-crate/Cargo.toml",
                "targets": [{ "name": "renamed_lib", "kind": ["lib"] }],
            }],
        });

        let resolved = resolve_dependency(&metadata, "some_crate").expect("dependency resolved");
        assert_eq!(resolved.lib_name, "renamed_lib");
    }

    #[test]
    fn ignores_non_library_targets_when_choosing_lib_name() {
        let metadata = json!({
            "packages": [{
                "name": "tool",
                "manifest_path": "/deps/tool/Cargo.toml",
                "targets": [
                    { "name": "tool", "kind": ["bin"] },
                    { "name": "tool_lib", "kind": ["rlib"] },
                ],
            }],
        });

        let resolved = resolve_dependency(&metadata, "tool").expect("dependency resolved");
        assert_eq!(resolved.lib_name, "tool_lib");
    }

    #[test]
    fn resolves_proc_macro_library_target() {
        let metadata = json!({
            "packages": [{
                "name": "derive-macro",
                "manifest_path": "/deps/derive-macro/Cargo.toml",
                "targets": [{ "name": "derive_macro", "kind": ["proc-macro"] }],
            }],
        });

        let resolved = resolve_dependency(&metadata, "derive_macro").expect("dependency resolved");
        assert_eq!(resolved.lib_name, "derive_macro");
    }

    #[test]
    fn falls_back_to_normalized_ident_without_library_target() {
        let metadata = json!({
            "packages": [{
                "name": "bin-only",
                "manifest_path": "/deps/bin-only/Cargo.toml",
                "targets": [{ "name": "bin-only", "kind": ["bin"] }],
            }],
        });

        let resolved = resolve_dependency(&metadata, "bin_only").expect("dependency resolved");
        assert_eq!(resolved.lib_name, "bin_only");
    }

    #[test]
    fn returns_none_for_unknown_crate() {
        let metadata = json!({
            "packages": [{
                "name": "other",
                "manifest_path": "/deps/other/Cargo.toml",
                "targets": [{ "name": "other", "kind": ["lib"] }],
            }],
        });

        assert!(resolve_dependency(&metadata, "some_crate").is_none());
    }
}
