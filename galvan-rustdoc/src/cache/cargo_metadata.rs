use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

use crate::RustdocError;

pub(super) fn dependency_manifest_path(crate_name: &str) -> Result<Option<PathBuf>, RustdocError> {
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
    let manifest_path = metadata
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|package| package.get("name").and_then(Value::as_str) == Some(crate_name))
        .and_then(|package| package.get("manifest_path"))
        .and_then(Value::as_str)
        .map(PathBuf::from);

    Ok(manifest_path)
}
