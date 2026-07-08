use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

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
    /// Cache invalidation signal for the resolved package version/source.
    pub(super) fingerprint: String,
}

/// Target kinds that rustdoc can document as a library. A package has at most
/// one such target; everything else (`bin`, `example`, `test`, ...) is ignored.
const LIBRARY_KINDS: &[&str] = &["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"];

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
    let fingerprint =
        dependency_fingerprint(metadata, package, crate_name, &manifest_path, &lib_name);

    Some(ResolvedDependency {
        manifest_path,
        lib_name,
        fingerprint,
    })
}

fn dependency_fingerprint(
    metadata: &Value,
    package: &Value,
    crate_name: &str,
    manifest_path: &Path,
    lib_name: &str,
) -> String {
    let package_name = package
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(crate_name);
    let version = package
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let source = package.get("source").and_then(Value::as_str);

    let mut parts = vec![
        format!("crate={crate_name}"),
        format!("package={package_name}"),
        format!("lib={lib_name}"),
        format!("version={version}"),
        format!("source={}", source.unwrap_or("path")),
    ];

    if let Some(lock_entry) = cargo_lock_entry(metadata, package_name, version, source) {
        parts.push(format!("lock={lock_entry}"));
    }

    if source.is_none() || source.is_some_and(|source| source.starts_with("git+")) {
        parts.push(format!("manifest={}", manifest_path.display()));
        if let Some(mtime) = path_mtime(manifest_path) {
            parts.push(format!("manifest_mtime={mtime}"));
        }
        if let Some(source_dir) = manifest_path.parent() {
            parts.push(format!("source_dir={}", source_dir.display()));
            if let Some(mtime) = path_mtime(source_dir) {
                parts.push(format!("source_dir_mtime={mtime}"));
            }
        }
    }

    parts.join("\n")
}

fn cargo_lock_entry(
    metadata: &Value,
    package_name: &str,
    version: &str,
    source: Option<&str>,
) -> Option<String> {
    let workspace_root = metadata.get("workspace_root").and_then(Value::as_str)?;
    let lock = fs::read_to_string(Path::new(workspace_root).join("Cargo.lock")).ok()?;
    find_cargo_lock_entry(&lock, package_name, version, source)
}

fn find_cargo_lock_entry(
    lock: &str,
    package_name: &str,
    version: &str,
    source: Option<&str>,
) -> Option<String> {
    lock.split("[[package]]")
        .skip(1)
        .map(str::trim)
        .find(|entry| {
            lock_field(entry, "name") == Some(package_name)
                && lock_field(entry, "version") == Some(version)
                && lock_field(entry, "source") == source
        })
        .map(|entry| format!("[[package]]\n{entry}"))
}

fn lock_field<'a>(entry: &'a str, field: &str) -> Option<&'a str> {
    let prefix = format!("{field} = ");
    entry.lines().find_map(|line| {
        let value = line.trim().strip_prefix(&prefix)?;
        value.strip_prefix('"')?.strip_suffix('"')
    })
}

fn path_mtime(path: &Path) -> Option<String> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(format!(
        "{}.{:09}",
        duration.as_secs(),
        duration.subsec_nanos()
    ))
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
        assert!(resolved.fingerprint.contains("version=unknown"));
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
    fn fingerprint_includes_version_and_source() {
        let metadata = json!({
            "packages": [{
                "name": "some-crate",
                "version": "1.2.3",
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                "manifest_path": "/deps/some-crate/Cargo.toml",
                "targets": [{ "name": "some_crate", "kind": ["lib"] }],
            }],
        });

        let resolved = resolve_dependency(&metadata, "some_crate").expect("dependency resolved");

        assert!(resolved.fingerprint.contains("version=1.2.3"));
        assert!(resolved
            .fingerprint
            .contains("source=registry+https://github.com/rust-lang/crates.io-index"));
    }

    #[test]
    fn finds_matching_cargo_lock_entry() {
        let lock = r#"
[[package]]
name = "other"
version = "1.0.0"

[[package]]
name = "some-crate"
version = "1.2.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abc"
"#;

        let entry = find_cargo_lock_entry(
            lock,
            "some-crate",
            "1.2.3",
            Some("registry+https://github.com/rust-lang/crates.io-index"),
        )
        .expect("lock entry found");

        assert!(entry.contains("name = \"some-crate\""));
        assert!(entry.contains("checksum = \"abc\""));
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
