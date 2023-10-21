mod declaration;
mod modifier;
mod parse;
mod result;
mod source;
mod tokenizer;

pub use declaration::*;
pub use modifier::*;
pub use parse::*;
pub use result::*;
pub use source::*;
pub use tokenizer::*;

#[cfg(feature = "exec")]
pub mod exec {
    use std::{env, path::Path};
    use walkdir::WalkDir;

    use crate::{ItemWithSource, ParsedSource, Result};

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
