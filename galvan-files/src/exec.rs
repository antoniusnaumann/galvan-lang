use std::path::Path;
use walkdir::WalkDir;
use crate::{GalvanFileExtension, Source, SourceResult};

pub fn read_sources(path: impl AsRef<Path>) -> impl Iterator<Item = SourceResult> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.has_galvan_extension())
        .map(Source::read)
}