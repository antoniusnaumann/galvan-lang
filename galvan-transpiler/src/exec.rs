use std::path::Path;

use galvan_files::read_sources;

use crate::{transpile, TranspileError, TranspileOutput};

pub fn transpile_dir(path: impl AsRef<Path>) -> Result<Vec<TranspileOutput>, TranspileError> {
    transpile(read_sources(path)?)
}

/// This is for use in macros and should not be used directly
pub mod __private {
    use super::*;

    use std::path::PathBuf;

    pub fn __setup_galvan() -> String {
        let transpiled = match transpile_dir("src") {
            Ok(output) => output,
            Err(e) => return e.to_string(),
        };

        let out_dir: PathBuf = std::env::var_os("OUT_DIR").unwrap().into();
        for file in transpiled {
            let path = out_dir.join(file.file_name.as_ref());
            std::fs::write(path, file.content.as_ref()).unwrap();
        }

        // TODO: Output warnings here
        "".to_string()
    }
}
