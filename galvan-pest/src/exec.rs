use std::{env, path::Path};
use galvan_files::read_sources;

use crate::*;

pub fn parse_current_dir() -> Vec<(ParseResult<'static>, Source)> {
    let current_dir = env::current_dir().unwrap();
    parse_dir(current_dir)
}

pub fn parse_dir(path: impl AsRef<Path>) -> Vec<(ParseResult<'static>, Source)> {
    read_sources(path)
        // This is quick and dirty test code
        .map(Box::new)
        .map(Box::leak)
        .map(Result::unwrap)
        .map(|s| (parse_source(s), s.clone()))
        .collect::<Vec<_>>()

    // TODO: Aggregate and print errors
}