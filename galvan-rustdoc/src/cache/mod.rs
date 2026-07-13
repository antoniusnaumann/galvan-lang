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
    manifest_dir: PathBuf,
    root: PathBuf,
}

/// The consumer project directory rustdoc metadata is resolved against when
/// none is given explicitly: `CARGO_MANIFEST_DIR` (set under cargo, e.g. in
/// build scripts) or the current directory.
pub(crate) fn env_manifest_dir() -> PathBuf {
    env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

impl RustdocCache {
    /// A cache rooted at an explicit consumer project directory (the directory
    /// containing its `Cargo.toml`). Callers run by cargo derive it from
    /// `CARGO_MANIFEST_DIR` via [`env_manifest_dir`]; others (most importantly
    /// the language server) pass the project they analyze.
    pub(crate) fn new_in(crate_name: &str, manifest_dir: &std::path::Path) -> Self {
        Self {
            crate_name: crate_name.into(),
            manifest_dir: manifest_dir.to_path_buf(),
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
        let dependency = match dependency_manifest_path(&self.crate_name, &self.manifest_dir) {
            Ok(Some(dependency)) => dependency,
            Ok(None) => {
                let error = RustdocError::DependencyNotFound(self.crate_name.clone());
                self.write_stderr(error.to_string());
                return Err(error);
            }
            Err(error) => {
                self.write_stderr(error.to_string());
                if self.json_path().is_some() {
                    return Ok(());
                }
                return Err(error);
            }
        };

        self.record_dependency_root(&dependency.manifest_path);
        if self.is_current(&dependency.fingerprint) {
            self.clear_diagnostics();
            return Ok(());
        }

        let target_dir = self.root.join("target");
        let output = run_rustdoc_json(&dependency.manifest_path, &dependency.features, &target_dir);

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
                self.write_stderr(error.to_string());
                Err(error)
            }
        }
    }

    fn cache_path(&self) -> PathBuf {
        self.root.join(format!("{}.json", self.crate_name))
    }

    /// Remember where the documented dependency's sources live, so relative
    /// rustdoc span paths (`src/ser.rs`) can be resolved to absolute ones.
    fn record_dependency_root(&self, manifest_path: &std::path::Path) {
        if let Some(dir) = manifest_path.parent() {
            let _ = fs::write(self.dependency_root_path(), dir.to_string_lossy().as_ref());
        }
    }

    /// The recorded source directory of the documented dependency, if any.
    pub(crate) fn dependency_root(&self) -> Option<PathBuf> {
        let root = PathBuf::from(fs::read_to_string(self.dependency_root_path()).ok()?);
        root.is_dir().then_some(root)
    }

    fn dependency_root_path(&self) -> PathBuf {
        self.root.join(format!("{}.rootdir", self.crate_name))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_cache_requires_json_and_matching_fingerprint() {
        let cache = test_cache("current");
        fs::create_dir_all(&cache.root).unwrap();

        assert!(!cache.is_current("abc"));

        fs::write(cache.cache_path(), "{}").unwrap();
        assert!(!cache.is_current("abc"));

        fs::write(cache.fingerprint_path(), "def").unwrap();
        assert!(!cache.is_current("abc"));

        fs::write(cache.fingerprint_path(), "abc").unwrap();
        assert!(cache.is_current("abc"));

        let _ = fs::remove_dir_all(cache.root);
    }

    #[test]
    fn dependency_not_found_is_returned_and_written_to_stderr() {
        let cache = test_cache("missing-dependency");
        let error = cache
            .update_if_needed()
            .expect_err("missing dependency should be reported");

        assert!(matches!(error, RustdocError::DependencyNotFound(_)));
        let stderr = fs::read_to_string(cache.root.join("definitely_missing_galvan_dep.stderr"))
            .expect("stderr diagnostic");
        assert!(stderr.contains("definitely_missing_galvan_dep"));

        let _ = fs::remove_dir_all(cache.root);
    }

    fn test_cache(name: &str) -> RustdocCache {
        RustdocCache {
            crate_name: "definitely_missing_galvan_dep".into(),
            manifest_dir: PathBuf::from("."),
            root: unique_temp_dir(name),
        }
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "galvan-rustdoc-cache-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }
}
