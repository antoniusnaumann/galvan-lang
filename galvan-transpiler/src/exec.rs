use std::path::Path;

use galvan_files::read_sources;

use crate::{transpile, TranspileError, TranspileOutput};

pub fn transpile_dir(path: impl AsRef<Path>) -> Result<Vec<TranspileOutput>, TranspileError> {
    transpile(read_sources(path)?)
}

/// This is for use in macros and should not be used directly
pub mod __private {
    use super::*;
    use std::fs;

    use std::path::PathBuf;

    pub fn __setup_galvan() -> String {
        let transpiled = match transpile_dir("src") {
            Ok(output) => output,
            Err(e) => return e.to_string(),
        };

        let out_dir: PathBuf = std::env::var_os("OUT_DIR").unwrap().into();
        let mod_dir = out_dir.join(galvan_module!());
        if let Err(e) = fs::create_dir(&mod_dir) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                panic!("Failed to create module directory: {}", e);
            }
        }

        for file in transpiled {
            let dir = if file.file_name.as_ref() == galvan_module!("rs") {
                &out_dir
            } else {
                &mod_dir
            };

            let path = dir.join(file.file_name.as_ref());
            fs::write(path, file.content.as_ref()).unwrap();
        }

        // TODO: Output warnings here
        "".to_string()
    }
}
