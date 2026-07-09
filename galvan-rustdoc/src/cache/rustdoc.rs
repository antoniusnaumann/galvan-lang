use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::RustdocError;

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
/// together. Override the rustup toolchain with `GALVAN_RUSTDOC_TOOLCHAIN` when
/// regenerating against a different pin (e.g. CI on a newer schema), or set
/// `GALVAN_RUSTDOC_COMMAND` to a nightly cargo executable when rustup is not
/// available.
const DEFAULT_RUSTDOC_TOOLCHAIN: &str = "nightly-2026-07-02";

const RUSTDOC_COMMAND_ENV: &str = "GALVAN_RUSTDOC_COMMAND";
const RUSTDOC_TOOLCHAIN_ENV: &str = "GALVAN_RUSTDOC_TOOLCHAIN";

#[derive(Clone, Debug, PartialEq, Eq)]
enum RustdocLauncher {
    Rustup { toolchain: String },
    Direct { cargo: String },
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
enum ToolchainError {
    RustupMissing,
    ToolchainNotInstalled(String),
}

fn rustdoc_toolchain() -> String {
    rustdoc_toolchain_from_env(env::var(RUSTDOC_TOOLCHAIN_ENV).ok().as_deref())
}

fn rustdoc_toolchain_from_env(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| DEFAULT_RUSTDOC_TOOLCHAIN.to_string())
}

fn rustdoc_command_from_env(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn toolchain_installed(toolchain_list: &str, toolchain: &str) -> bool {
    toolchain_list.lines().any(|line| {
        let installed = line.split_whitespace().next().unwrap_or_default();
        installed == toolchain || installed.starts_with(&format!("{toolchain}-"))
    })
}

#[cfg(test)]
fn resolve_launcher_from_toolchain_list(
    command_env: Option<&str>,
    toolchain_env: Option<&str>,
    toolchain_list: Option<&str>,
) -> Result<RustdocLauncher, ToolchainError> {
    if let Some(cargo) = rustdoc_command_from_env(command_env) {
        return Ok(RustdocLauncher::Direct { cargo });
    }

    let toolchain = rustdoc_toolchain_from_env(toolchain_env);
    let Some(toolchain_list) = toolchain_list else {
        return Err(ToolchainError::RustupMissing);
    };
    if toolchain_installed(toolchain_list, &toolchain) {
        Ok(RustdocLauncher::Rustup { toolchain })
    } else {
        Err(ToolchainError::ToolchainNotInstalled(toolchain))
    }
}

fn resolve_launcher() -> Result<RustdocLauncher, RustdocError> {
    if let Some(cargo) = rustdoc_command_from_env(env::var(RUSTDOC_COMMAND_ENV).ok().as_deref()) {
        return Ok(RustdocLauncher::Direct { cargo });
    }

    let toolchain = rustdoc_toolchain();
    let output = Command::new("rustup")
        .arg("toolchain")
        .arg("list")
        .output()
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => RustdocError::ToolchainUnavailable,
            _ => RustdocError::RustdocSpawn(error),
        })?;

    if toolchain_installed(&String::from_utf8_lossy(&output.stdout), &toolchain) {
        Ok(RustdocLauncher::Rustup { toolchain })
    } else {
        Err(RustdocError::ToolchainNotInstalled(toolchain))
    }
}

pub(super) fn run_rustdoc_json(
    manifest_path: &Path,
    target_dir: &Path,
) -> Result<Output, RustdocError> {
    let launcher = resolve_launcher()?;
    let mut command = match launcher {
        RustdocLauncher::Rustup { toolchain } => {
            let mut command = Command::new("rustup");
            command.arg("run").arg(toolchain).arg("cargo");
            command
        }
        RustdocLauncher::Direct { cargo } => Command::new(cargo),
    };

    command
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
        .map_err(RustdocError::RustdocSpawn)
}

/// rustdoc names its JSON output after the library target, not the package or
/// the Galvan crate ident, so callers must pass the resolved lib name.
pub(super) fn generated_json_path(lib_name: &str, target_dir: &Path) -> PathBuf {
    target_dir.join("doc").join(format!("{lib_name}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_override_takes_precedence_over_missing_rustup() {
        let launcher =
            resolve_launcher_from_toolchain_list(Some("/opt/nightly/bin/cargo"), None, None)
                .expect("command override should bypass rustup detection");

        assert_eq!(
            launcher,
            RustdocLauncher::Direct {
                cargo: "/opt/nightly/bin/cargo".to_string(),
            }
        );
    }

    #[test]
    fn blank_command_override_is_ignored() {
        let launcher =
            resolve_launcher_from_toolchain_list(Some("  "), None, Some("nightly-2026-07-02\n"))
                .expect("blank command override should not bypass rustup");

        assert_eq!(
            launcher,
            RustdocLauncher::Rustup {
                toolchain: "nightly-2026-07-02".to_string(),
            }
        );
    }

    #[test]
    fn installed_toolchain_resolves_to_rustup_launcher() {
        let list = "\
stable-aarch64-apple-darwin (default)
nightly-2026-07-02-aarch64-apple-darwin
";
        let launcher = resolve_launcher_from_toolchain_list(None, None, Some(list))
            .expect("default toolchain should be detected");

        assert_eq!(
            launcher,
            RustdocLauncher::Rustup {
                toolchain: "nightly-2026-07-02".to_string(),
            }
        );
    }

    #[test]
    fn custom_toolchain_override_is_used_for_rustup_launcher() {
        let list = "\
stable-aarch64-apple-darwin (default)
nightly-2026-07-09-aarch64-apple-darwin
";
        let launcher =
            resolve_launcher_from_toolchain_list(None, Some("nightly-2026-07-09"), Some(list))
                .expect("custom toolchain should be detected");

        assert_eq!(
            launcher,
            RustdocLauncher::Rustup {
                toolchain: "nightly-2026-07-09".to_string(),
            }
        );
    }

    #[test]
    fn missing_rustup_is_reported_without_command_override() {
        let error = resolve_launcher_from_toolchain_list(None, None, None)
            .expect_err("missing rustup should be reported");

        assert_eq!(error, ToolchainError::RustupMissing);
    }

    #[test]
    fn missing_toolchain_names_requested_toolchain() {
        let error = resolve_launcher_from_toolchain_list(
            None,
            Some("nightly-2026-07-09"),
            Some("nightly-2026-07-02-aarch64-apple-darwin\n"),
        )
        .expect_err("missing toolchain should be reported");

        assert_eq!(
            error,
            ToolchainError::ToolchainNotInstalled("nightly-2026-07-09".to_string())
        );
    }
}
