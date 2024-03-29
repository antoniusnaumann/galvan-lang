use crate::{FileError, GalvanFileExtension, Source};
use std::path::Path;
use walkdir::WalkDir;

pub fn read_sources(path: impl AsRef<Path>, filter: Vec<String>) -> Result<Vec<Source>, FileError> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.has_galvan_extension())
        .filter(|p| {
            filter.is_empty()
                || filter.contains(&p.file_name().unwrap().to_str().unwrap().to_string())
        })
        .map(Source::read)
        .collect()
}
