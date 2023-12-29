use galvan_files::read_sources;
use std::{env, path::Path};

use crate::*;

pub fn parse_current_dir() -> Vec<(ParseResult<'static>, Source)> {
    let current_dir = env::current_dir().unwrap();
    parse_dir(current_dir)
}

pub fn parse_dir(path: impl AsRef<Path>) -> Vec<(ParseResult<'static>, Source)> {
    read_sources(path)
        .unwrap()
        .into_iter()
        .map(|s| (parse_source(Box::leak(Box::new(s.clone()))), s))
        .collect::<Vec<_>>()
    // TODO: Aggregate and print errors
}
