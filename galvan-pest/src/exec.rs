use std::{env, path::Path};
use walkdir::WalkDir;

use crate::*;

pub fn parse_current_dir() -> Vec<(ParseResult<'static>, Source)> {
    let current_dir = env::current_dir().unwrap();
    parse_dir(current_dir)
}

pub fn parse_dir(path: impl AsRef<Path>) -> Vec<(ParseResult<'static>, Source)> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension() == Some("galvan".as_ref()))
        .map(Source::read)
        .map(Box::new)
        .map(Box::leak)
        .map(|s| (parse_source(s), s.clone()))
        .collect::<Vec<_>>()


    // TODO: Aggregate and print errors
}