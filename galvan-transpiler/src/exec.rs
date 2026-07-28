use std::path::Path;

use galvan_files::read_sources;

use crate::{transpile, TranspileError, TranspileOutput, TranspileResult};

pub fn transpile_dir(
    path: impl AsRef<Path>,
    filter: Vec<String>,
) -> Result<Vec<TranspileOutput>, TranspileError> {
    transpile(read_sources(path, filter)?)
}

pub(crate) fn transpile_dir_with_rustdoc_warnings(
    path: impl AsRef<Path>,
    filter: Vec<String>,
    rustdoc_warning: impl FnMut(&crate::RustdocError),
) -> Result<TranspileResult, TranspileError> {
    crate::transpile_sources_with_diagnostics_and_rustdoc_warnings(
        read_sources(path, filter)?,
        rustdoc_warning,
    )
}

/// This is for use in macros and should not be used directly
pub mod __private {
    use super::*;
    use std::fs;

    use std::path::PathBuf;

    pub fn __setup_galvan() -> String {
        let transpiled = match transpile_dir_with_rustdoc_warnings("src", vec![], |warning| {
            println!("cargo::warning={warning}");
        }) {
            Ok(output) => output,
            Err(e) => return e.to_string(),
        };
        for diagnostic in &transpiled.diagnostics {
            match diagnostic.severity {
                crate::DiagnosticSeverity::Error => {
                    println!("cargo::error={}", diagnostic.message);
                }
                crate::DiagnosticSeverity::Warning => {
                    println!("cargo::warning={}", diagnostic.message);
                }
                crate::DiagnosticSeverity::Info => {}
            }
        }
        if transpiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::DiagnosticSeverity::Error)
        {
            return "Galvan transpilation failed; see diagnostics above".to_string();
        }

        let out_dir: PathBuf = std::env::var_os("OUT_DIR").unwrap().into();
        let mod_dir = out_dir.join(galvan_module!());
        if let Err(e) = fs::create_dir(&mod_dir) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                panic!("Failed to create module directory: {}", e);
            }
        }

        for file in transpiled.outputs {
            let dir = if file.file_name.as_ref() == galvan_module!("rs") {
                &out_dir
            } else {
                &mod_dir
            };

            let path = dir.join(file.file_name.as_ref());
            fs::write(path, file.content.as_ref()).unwrap();
        }
        "".to_string()
    }
}
