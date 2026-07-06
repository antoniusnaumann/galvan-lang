use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The nightly toolchain used to generate rustdoc JSON.
///
/// rustdoc JSON output is a nightly-only, unstable feature (it requires
/// `-Z unstable-options`; stable rejects it outright), and every toolchain emits
/// exactly one hardcoded `format_version` — there is no flag to request a
/// specific one. A *floating* `nightly` therefore drifts: the schema can bump
/// several versions within days, silently outrunning the `rustdoc-types` pin.
///
/// So we pin an exact nightly date that emits the schema
/// [`crate::interop`] deserializes into. This is the other half of the
/// coordinated pin documented on `RUSTDOC_FORMAT_VERSION`: this date and the
/// `rustdoc-types = "=0.60.0"` dependency (`FORMAT_VERSION == 60`) must move
/// together. Override with `GALVAN_RUSTDOC_TOOLCHAIN` when regenerating against
/// a different pin (e.g. CI on a newer schema).
const DEFAULT_RUSTDOC_TOOLCHAIN: &str = "nightly-2026-07-02";

fn rustdoc_toolchain() -> String {
    env::var("GALVAN_RUSTDOC_TOOLCHAIN")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_RUSTDOC_TOOLCHAIN.to_string())
}

pub(super) fn run_rustdoc_json(manifest_path: &Path, target_dir: &Path) -> std::io::Result<Output> {
    Command::new("rustup")
        .arg("run")
        .arg(rustdoc_toolchain())
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
