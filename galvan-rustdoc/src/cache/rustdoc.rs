use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(super) fn run_rustdoc_json(manifest_path: &Path, target_dir: &Path) -> std::io::Result<Output> {
    Command::new("rustup")
        .arg("run")
        .arg("nightly")
        .arg("cargo")
        .arg("rustdoc")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--lib")
        .arg("--target-dir")
        .arg(target_dir)
        .arg("--")
        .arg("-Z")
        .arg("unstable-options")
        .arg("--output-format")
        .arg("json")
        .env("GALVAN_RUSTDOC_CACHE_UPDATING", "1")
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .env_remove("RUSTC_WRAPPER")
        .output()
}

/// rustdoc names its JSON output after the library target, not the package or
/// the Galvan crate ident, so callers must pass the resolved lib name.
pub(super) fn generated_json_path(lib_name: &str, target_dir: &Path) -> PathBuf {
    target_dir.join("doc").join(format!("{lib_name}.json"))
}
