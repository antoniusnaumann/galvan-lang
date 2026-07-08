mod cargo_metadata;
mod rustdoc;

use std::env;
use std::fs;
use std::path::PathBuf;

use self::cargo_metadata::dependency_manifest_path;
use self::rustdoc::{generated_json_path, run_rustdoc_json};
use crate::RustdocError;

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
        let path = self.cache_path();
        path.exists().then_some(path)
    }

    pub(crate) fn update_if_needed(&self) -> Result<(), RustdocError> {
        if env::var_os("GALVAN_RUSTDOC_CACHE_UPDATING").is_some() {
            return Ok(());
        }

        let _ = fs::create_dir_all(&self.root);
        let dependency = match dependency_manifest_path(&self.crate_name) {
            Ok(Some(dependency)) => dependency,
            Ok(None) => {
                let error = RustdocError::DependencyNotFound(self.crate_name.clone());
                self.write_stderr(error.to_string());
                return Err(error);
            }
            Err(error) => {
                self.write_stderr(error.to_string());
                return Err(error);
            }
        };

        if self.is_current(&dependency.fingerprint) {
            self.clear_diagnostics();
            return Ok(());
        }

        let target_dir = self.root.join("target");
        let output = run_rustdoc_json(&dependency.manifest_path, &target_dir);

        match output {
            Ok(output) if output.status.success() => {
                let generated = generated_json_path(&dependency.lib_name, &target_dir);
                let cached = self.cache_path();
                if !generated.exists() {
                    self.write_stderr(format!(
                        "rustdoc succeeded but {} was not found\n{}",
                        generated.display(),
                        String::from_utf8_lossy(&output.stderr)
                    ));
                    return Err(RustdocError::GeneratedJsonMissing(generated));
                }
                fs::copy(&generated, &cached)
                    .map_err(|error| RustdocError::WriteCache(cached.clone(), error))?;
                fs::write(self.fingerprint_path(), &dependency.fingerprint)
                    .map_err(|error| RustdocError::WriteCache(self.fingerprint_path(), error))?;
                self.clear_diagnostics();
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                self.write_stderr(&stderr);
                let _ = fs::write(
                    self.root.join(format!("{}.stdout", self.crate_name)),
                    String::from_utf8_lossy(&output.stdout).as_ref(),
                );
                Err(RustdocError::RustdocGeneration {
                    crate_name: self.crate_name.clone(),
                    stderr,
                })
            }
            Err(error) => {
                let error = RustdocError::RustdocSpawn(error);
                self.write_stderr(error.to_string());
                Err(error)
            }
        }
    }

    fn cache_path(&self) -> PathBuf {
        self.root.join(format!("{}.json", self.crate_name))
    }

    fn fingerprint_path(&self) -> PathBuf {
        self.root.join(format!("{}.fingerprint", self.crate_name))
    }

    fn is_current(&self, fingerprint: &str) -> bool {
        self.cache_path().exists()
            && fs::read_to_string(self.fingerprint_path()).is_ok_and(|cached| cached == fingerprint)
    }

    fn write_stderr(&self, stderr: impl AsRef<str>) {
        let _ = fs::write(
            self.root.join(format!("{}.stderr", self.crate_name)),
            stderr.as_ref(),
        );
    }

    fn clear_diagnostics(&self) {
        let _ = fs::remove_file(self.root.join(format!("{}.stderr", self.crate_name)));
        let _ = fs::remove_file(self.root.join(format!("{}.stdout", self.crate_name)));
    }
}
