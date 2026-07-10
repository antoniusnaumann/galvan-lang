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
    /// Features Cargo resolved for this exact package node.
    pub(super) features: Vec<String>,
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
        .arg(&manifest_path)
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .env_remove("RUSTC_WRAPPER")
        .output()
        .map_err(RustdocError::CargoMetadata)?;
    if !output.status.success() {
        return Err(RustdocError::CargoMetadataFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    let metadata: Value =
        serde_json::from_slice(&output.stdout).map_err(RustdocError::InvalidCargoMetadata)?;

    Ok(resolve_dependency(&metadata, &manifest_path, crate_name))
}

/// Resolves the exact dependency edge used by the consuming package. The edge
/// name includes Cargo dependency renames, while its package id disambiguates
/// multiple resolved versions of the same package.
fn resolve_dependency(
    metadata: &Value,
    consumer_manifest_path: &Path,
    crate_name: &str,
) -> Option<ResolvedDependency> {
    let consumer_id = consumer_package_id(metadata, consumer_manifest_path)?;
    let dependency_id = resolve_nodes(metadata)
        .find(|node| node.get("id").and_then(Value::as_str) == Some(consumer_id))?
        .get("deps")?
        .as_array()?
        .iter()
        .find(|dependency| {
            dependency
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| normalize_crate_name(name) == crate_name)
        })?
        .get("pkg")?
        .as_str()?;

    let package = metadata
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|package| package.get("id").and_then(Value::as_str) == Some(dependency_id))?;

    let manifest_path = package
        .get("manifest_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)?;

    // rustdoc names its JSON after the library target. Fall back to the
    // normalized package name when no library target is reported (in which case
    // `cargo rustdoc --lib` will fail loudly downstream anyway).
    let lib_name = library_target_name(package).unwrap_or_else(|| crate_name.to_string());
    let mut features = resolve_nodes(metadata)
        .find(|node| node.get("id").and_then(Value::as_str) == Some(dependency_id))
        .and_then(|node| node.get("features"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    features.sort();
    features.dedup();
    let fingerprint = dependency_fingerprint(
        metadata,
        package,
        crate_name,
        dependency_id,
        &features,
        &manifest_path,
        &lib_name,
    );

    Some(ResolvedDependency {
        manifest_path,
        lib_name,
        features,
        fingerprint,
    })
}

fn consumer_package_id<'a>(metadata: &'a Value, manifest_path: &Path) -> Option<&'a str> {
    metadata
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|package| {
            package
                .get("manifest_path")
                .and_then(Value::as_str)
                .is_some_and(|path| Path::new(path) == manifest_path)
        })
        .and_then(|package| package.get("id"))
        .and_then(Value::as_str)
        .or_else(|| {
            metadata
                .get("resolve")
                .and_then(|resolve| resolve.get("root"))
                .and_then(Value::as_str)
        })
}

fn resolve_nodes(metadata: &Value) -> impl Iterator<Item = &Value> {
    metadata
        .get("resolve")
        .and_then(|resolve| resolve.get("nodes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn dependency_fingerprint(
    metadata: &Value,
    package: &Value,
    crate_name: &str,
    package_id: &str,
    features: &[String],
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
        format!("package_id={package_id}"),
        format!("package={package_name}"),
        format!("lib={lib_name}"),
        format!("version={version}"),
        format!("source={}", source.unwrap_or("path")),
        format!("features={}", features.join(",")),
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
            if let Some(mtime) = source_tree_max_mtime(source_dir) {
                parts.push(format!("source_tree_mtime={mtime}"));
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

fn source_tree_max_mtime(root: &Path) -> Option<String> {
    let max = max_file_modified(root)?;
    let duration = max.duration_since(UNIX_EPOCH).ok()?;
    Some(format!(
        "{}.{:09}",
        duration.as_secs(),
        duration.subsec_nanos()
    ))
}

fn max_file_modified(path: &Path) -> Option<std::time::SystemTime> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.is_file() {
        return metadata.modified().ok();
    }
    if !metadata.is_dir() {
        return None;
    }

    let mut max: Option<std::time::SystemTime> = None;
    for entry in fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        let file_name = entry.file_name();
        if file_name == ".git" || file_name == "target" {
            continue;
        }
        if let Some(modified) = max_file_modified(&entry.path()) {
            max = Some(match max {
                Some(max) => max.max(modified),
                None => modified,
            });
        }
    }
    max
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

    const CONSUMER_MANIFEST: &str = "/workspace/app/Cargo.toml";

    fn metadata_for(package: Value, dependency_name: &str, features: &[&str]) -> Value {
        let package_name = package["name"].as_str().expect("package name");
        let version = package["version"].as_str().unwrap_or("0.0.0");
        let package_id = format!("registry+https://example.invalid#{package_name}@{version}");
        let mut package = package;
        package["id"] = Value::String(package_id.clone());
        json!({
            "packages": [
                {
                    "id": "path+file:///workspace/app#0.1.0",
                    "name": "app",
                    "manifest_path": CONSUMER_MANIFEST,
                    "targets": [{ "name": "app", "kind": ["bin"] }]
                },
                package
            ],
            "resolve": {
                "root": "path+file:///workspace/app#0.1.0",
                "nodes": [
                    {
                        "id": "path+file:///workspace/app#0.1.0",
                        "deps": [{ "name": dependency_name, "pkg": package_id }],
                        "features": []
                    },
                    {
                        "id": package_id,
                        "deps": [],
                        "features": features
                    }
                ]
            }
        })
    }

    fn resolve(metadata: &Value, crate_name: &str) -> Option<ResolvedDependency> {
        resolve_dependency(metadata, Path::new(CONSUMER_MANIFEST), crate_name)
    }

    #[test]
    fn matches_dashed_package_by_underscored_ident() {
        let metadata = metadata_for(
            json!({
                "name": "some-crate",
                "version": "1.0.0",
                "manifest_path": "/deps/some-crate/Cargo.toml",
                "targets": [{ "name": "some_crate", "kind": ["lib"] }],
            }),
            "some_crate",
            &[],
        );

        let resolved = resolve(&metadata, "some_crate").expect("dependency resolved");
        assert_eq!(
            resolved.manifest_path,
            PathBuf::from("/deps/some-crate/Cargo.toml")
        );
        assert_eq!(resolved.lib_name, "some_crate");
        assert!(resolved.fingerprint.contains("version=1.0.0"));
    }

    #[test]
    fn prefers_explicit_lib_name_over_package_name() {
        let metadata = metadata_for(
            json!({
                "name": "some-crate",
                "version": "1.0.0",
                "manifest_path": "/deps/some-crate/Cargo.toml",
                "targets": [{ "name": "renamed_lib", "kind": ["lib"] }],
            }),
            "some_crate",
            &[],
        );

        let resolved = resolve(&metadata, "some_crate").expect("dependency resolved");
        assert_eq!(resolved.lib_name, "renamed_lib");
    }

    #[test]
    fn resolves_renamed_dependency_by_edge_name() {
        let metadata = metadata_for(
            json!({
                "name": "serde_json",
                "version": "1.0.0",
                "manifest_path": "/deps/serde_json/Cargo.toml",
                "targets": [{ "name": "serde_json", "kind": ["lib"] }],
            }),
            "json",
            &[],
        );

        let resolved = resolve(&metadata, "json").expect("renamed dependency resolved");
        assert_eq!(resolved.lib_name, "serde_json");
        assert!(resolve(&metadata, "serde_json").is_none());
        assert!(resolved.fingerprint.contains("crate=json"));
    }

    #[test]
    fn package_id_disambiguates_multiple_versions() {
        let metadata = json!({
            "packages": [
                {
                    "id": "path+file:///workspace/app#0.1.0",
                    "name": "app",
                    "manifest_path": CONSUMER_MANIFEST,
                    "targets": [{ "name": "app", "kind": ["bin"] }]
                },
                {
                    "id": "registry+https://example.invalid#shared@1.0.0",
                    "name": "shared",
                    "version": "1.0.0",
                    "manifest_path": "/deps/shared-1/Cargo.toml",
                    "targets": [{ "name": "shared", "kind": ["lib"] }]
                },
                {
                    "id": "registry+https://example.invalid#shared@2.0.0",
                    "name": "shared",
                    "version": "2.0.0",
                    "manifest_path": "/deps/shared-2/Cargo.toml",
                    "targets": [{ "name": "shared", "kind": ["lib"] }]
                }
            ],
            "resolve": {
                "root": "path+file:///workspace/app#0.1.0",
                "nodes": [
                    {
                        "id": "path+file:///workspace/app#0.1.0",
                        "deps": [{
                            "name": "shared_v2",
                            "pkg": "registry+https://example.invalid#shared@2.0.0"
                        }],
                        "features": []
                    },
                    {
                        "id": "registry+https://example.invalid#shared@1.0.0",
                        "deps": [],
                        "features": []
                    },
                    {
                        "id": "registry+https://example.invalid#shared@2.0.0",
                        "deps": [],
                        "features": []
                    }
                ]
            }
        });

        let resolved = resolve(&metadata, "shared_v2").expect("dependency resolved");
        assert_eq!(
            resolved.manifest_path,
            PathBuf::from("/deps/shared-2/Cargo.toml")
        );
        assert!(resolved
            .fingerprint
            .contains("package_id=registry+https://example.invalid#shared@2.0.0"));
    }

    #[test]
    fn fingerprint_includes_version_source_and_features() {
        let metadata = metadata_for(
            json!({
                "name": "some-crate",
                "version": "1.2.3",
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                "manifest_path": "/deps/some-crate/Cargo.toml",
                "targets": [{ "name": "some_crate", "kind": ["lib"] }],
            }),
            "some_crate",
            &["std", "default", "std"],
        );

        let resolved = resolve(&metadata, "some_crate").expect("dependency resolved");

        assert_eq!(resolved.features, vec!["default", "std"]);
        assert!(resolved.fingerprint.contains("version=1.2.3"));
        assert!(resolved
            .fingerprint
            .contains("source=registry+https://github.com/rust-lang/crates.io-index"));
        assert!(resolved.fingerprint.contains("features=default,std"));
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
    fn source_tree_mtime_tracks_nested_file_edits() {
        let root = unique_temp_dir("source-tree-mtime");
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("lib.rs"), "pub fn first() {}").unwrap();
        let first = source_tree_max_mtime(&root).expect("source mtime");

        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(src.join("lib.rs"), "pub fn second() {}").unwrap();
        let second = source_tree_max_mtime(&root).expect("updated source mtime");

        assert_ne!(first, second);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ignores_non_library_targets_when_choosing_lib_name() {
        let metadata = metadata_for(
            json!({
                "name": "tool",
                "version": "1.0.0",
                "manifest_path": "/deps/tool/Cargo.toml",
                "targets": [
                    { "name": "tool", "kind": ["bin"] },
                    { "name": "tool_lib", "kind": ["rlib"] },
                ],
            }),
            "tool",
            &[],
        );

        let resolved = resolve(&metadata, "tool").expect("dependency resolved");
        assert_eq!(resolved.lib_name, "tool_lib");
    }

    #[test]
    fn resolves_proc_macro_library_target() {
        let metadata = metadata_for(
            json!({
                "name": "derive-macro",
                "version": "1.0.0",
                "manifest_path": "/deps/derive-macro/Cargo.toml",
                "targets": [{ "name": "derive_macro", "kind": ["proc-macro"] }],
            }),
            "derive_macro",
            &[],
        );

        let resolved = resolve(&metadata, "derive_macro").expect("dependency resolved");
        assert_eq!(resolved.lib_name, "derive_macro");
    }

    #[test]
    fn falls_back_to_normalized_ident_without_library_target() {
        let metadata = metadata_for(
            json!({
                "name": "bin-only",
                "version": "1.0.0",
                "manifest_path": "/deps/bin-only/Cargo.toml",
                "targets": [{ "name": "bin-only", "kind": ["bin"] }],
            }),
            "bin_only",
            &[],
        );

        let resolved = resolve(&metadata, "bin_only").expect("dependency resolved");
        assert_eq!(resolved.lib_name, "bin_only");
    }

    #[test]
    fn returns_none_for_unknown_crate() {
        let metadata = metadata_for(
            json!({
                "name": "other",
                "version": "1.0.0",
                "manifest_path": "/deps/other/Cargo.toml",
                "targets": [{ "name": "other", "kind": ["lib"] }],
            }),
            "other",
            &[],
        );

        assert!(resolve(&metadata, "some_crate").is_none());
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "galvan-rustdoc-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }
}
