use std::path::Path;

use galvan_files::read_sources;

use crate::{transpile, RustSource};

pub fn transpile_dir(path: impl AsRef<Path>) -> impl Iterator<Item = RustSource> {
    read_sources(path)
        // TODO: handle errors
        .map(|s| s.unwrap_or_else(|e| panic!("{}", e)))
        .map(transpile)
}

/// This is for use in macros and should not be used directly
pub mod __private {
    use super::*;
    use std::path::PathBuf;

    type Warnings = Vec<String>;
    pub fn __setup_galvan() -> Warnings {
        // TODO: Refactor, this can be done by grouping the results into (source, transpiled) and (source, errors) pairs
        let mut output = transpile_dir("src");
        let mut msgs = Vec::new();

        if output.any(|s| s.has_errors()) {
            for result in output {
                let errors = result.errors();
                if errors.is_empty() {
                    continue;
                }

                // TODO: Use relavtive path here
                let mut msg = format!("{}", result.source.origin().unwrap().to_string_lossy());
                for e in errors.errors {
                    msg.push_str(&format!("\n{}", e));
                }
                msgs.push(msg);
            }

            return msgs;
        }

        let out_dir: PathBuf = std::env::var_os("OUT_DIR").unwrap().into();
        // let out_dir: PathBuf = "src".into();

        for RustSource { transpiled, source } in output {
            let path = out_dir.join(source.rust_name().unwrap().as_str());
            std::fs::write(path, transpiled.unwrap()).unwrap();
        }

        Vec::new()
    }
}

#[macro_export]
macro_rules! setup_galvan {
    () => {
        let warnings = galvan_transpiler::exec::__private::__setup_galvan();

        println!("cargo:warning={}", warnings.join("\n"));
        // TODO: How to build a rerun rule for this?
    };
}
