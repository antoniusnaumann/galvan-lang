mod parse;
mod result;
mod tokenizer;

pub use galvan_ast::modifier::*;
pub use parse::*;
pub use result::*;
pub use galvan_pest::source::*;
pub use tokenizer::*;

pub use galvan_ast::*;

#[cfg(feature = "exec")]
pub mod exec {
    use std::{env, path::Path};
    use walkdir::WalkDir;

    use crate::{ParsedSource, Result};

    use super::Source;

    #[allow(clippy::redundant_closure)]
    pub fn parse_current_dir() -> Vec<(Result<ParsedSource>, Source)> {
        let current_dir = env::current_dir().unwrap();
        parse_dir(current_dir)
    }

    pub fn parse_dir(path: impl AsRef<Path>) -> Vec<(Result<ParsedSource>, Source)> {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| p.extension() == Some("galvan".as_ref()))
            .map(|p| Source::read(p))
            .map(|s| (super::parse_source(&s), s))
            .collect::<Vec<_>>()

        // TODO: Aggregate and print errors
    }
}
