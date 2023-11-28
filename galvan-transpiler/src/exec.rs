use std::path::Path;

use galvan_files::read_sources;

use crate::{transpile, Transpilation};

pub fn transpile_dir(path: impl AsRef<Path>) -> impl Iterator<Item = Transpilation> {
    read_sources(path)
        // TODO: handle errors
        .map(|s| s.unwrap_or_else(|e| panic!("{}", e)))
        .map(transpile)
}

/// This is for use in macros and should not be used directly
pub mod __private {
    use super::*;

    use crate::{galvan_module, SuccessfulTranspilation};
    use itertools::Itertools;
    use std::path::PathBuf;

    type TranspilerErrorMessages = Vec<String>;
    pub fn __setup_galvan() -> TranspilerErrorMessages {
        fn generate_galvan_mod(out_dir: &Path, mod_names: Vec<String>) {
            let mut mod_contents = String::new();
            for mod_name in mod_names {
                mod_contents.push_str(&format!("mod {};\npub use {}::*;\n", mod_name, mod_name));
            }

            let path = out_dir.join(galvan_module!());
            std::fs::write(path, mod_contents).unwrap();
        }

        // TODO: Refactor, this can be done by grouping the results into (source, transpiled) and (source, errors) pairs
        let (successes, failures): (Vec<_>, Vec<_>) =
            transpile_dir("src").map(|t| t.into()).partition_result();
        let mut msgs = Vec::new();

        if !failures.is_empty() {
            for result in failures {
                let errors = result.errors;

                // TODO: Use relative path here
                let mut msg = format!("{}", result.source.origin().unwrap().to_string_lossy());
                msg.push_str(&format!("\n{}", errors));
                msgs.push(msg);
            }

            return msgs;
        }

        let out_dir: PathBuf = std::env::var_os("OUT_DIR").unwrap().into();
        // let out_dir: PathBuf = "src".into();

        let mut mod_names = vec![];
        for SuccessfulTranspilation { transpiled, source } in successes {
            let rust_name = source.rust_name().unwrap();
            let path = out_dir.join(&rust_name);
            std::fs::write(path, transpiled).unwrap();

            mod_names.push(source.canonical_name().unwrap().to_string());
        }

        generate_galvan_mod(&out_dir, mod_names);

        msgs
    }
}

#[macro_export]
macro_rules! setup {
    () => {
        let warnings = galvan_transpiler::exec::__private::__setup_galvan();

        if !warnings.is_empty() {
            // println!("cargo:warning={}", warnings.join("\n"));
            panic!("Galvan Transpiler Error:\n{}", warnings.join("\n"));
        }
        // TODO: How to build a rerun rule for this?
    };
}
