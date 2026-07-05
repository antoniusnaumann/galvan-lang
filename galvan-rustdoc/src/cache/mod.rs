mod cargo_metadata;
mod rustdoc;

use std::env;
use std::fs;
use std::path::PathBuf;

use self::cargo_metadata::dependency_manifest_path;
use self::rustdoc::{generated_json_path, run_rustdoc_json};

pub(crate) struct RustdocCache {
    crate_name: Box<str>,
    root: PathBuf,
}

impl RustdocCache {
    pub(crate) fn new(crate_name: &str) -> Self {
        let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            crate_name: crate_name.into(),
            root: manifest_dir
                .join("target")
                .join("galvan")
                .join("rustdoc-json"),
        }
    }

    pub(crate) fn json_path(&self) -> Option<PathBuf> {
        let path = self.root.join(format!("{}.json", self.crate_name));
        path.exists().then_some(path)
    }

    pub(crate) fn update_if_needed(&self) {
        if self.json_path().is_some() {
            self.clear_diagnostics();
            return;
        }
        if env::var_os("GALVAN_RUSTDOC_CACHE_UPDATING").is_some() {
            return;
        }

        let _ = fs::create_dir_all(&self.root);
        let dependency = match dependency_manifest_path(&self.crate_name) {
            Ok(Some(dependency)) => dependency,
            Ok(None) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    format!(
                        "crate '{}' was not found in cargo metadata",
                        self.crate_name
                    ),
                );
                return;
            }
            Err(error) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    error.to_string(),
                );
                return;
            }
        };

        let target_dir = self.root.join("target");
        let output = run_rustdoc_json(&dependency.manifest_path, &target_dir);

        match output {
            Ok(output) if output.status.success() => {
                let generated = generated_json_path(&dependency.lib_name, &target_dir);
                let cached = self.root.join(format!("{}.json", self.crate_name));
                if fs::copy(&generated, &cached).is_ok() {
                    self.clear_diagnostics();
                } else {
                    let _ = fs::write(
                        self.root.join(format!("{}.stderr", self.crate_name)),
                        format!(
                            "rustdoc succeeded but {} was not found\n{}",
                            generated.display(),
                            String::from_utf8_lossy(&output.stderr)
                        ),
                    );
                }
            }
            Ok(output) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    String::from_utf8_lossy(&output.stderr).as_ref(),
                );
                let _ = fs::write(
                    self.root.join(format!("{}.stdout", self.crate_name)),
                    String::from_utf8_lossy(&output.stdout).as_ref(),
                );
            }
            Err(error) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    error.to_string(),
                );
            }
        }
    }

    fn clear_diagnostics(&self) {
        let _ = fs::remove_file(self.root.join(format!("{}.stderr", self.crate_name)));
        let _ = fs::remove_file(self.root.join(format!("{}.stdout", self.crate_name)));
    }
}
