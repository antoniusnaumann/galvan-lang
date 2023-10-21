use std::env;
use walkdir::WalkDir;

use galvan_parser::{ItemWithSource, Source};

#[allow(clippy::redundant_closure)]
fn main() {
    let current_dir = env::current_dir().unwrap();

    let src = WalkDir::new(current_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension() == Some("galvan".as_ref()))
        .map(|p| Source::read(p))
        .map(|s| (galvan_parser::parse_source(&s), s))
        .collect::<Vec<_>>();

    // TODO: Aggregate and print errors

    // TODO: Transpile to Rust
    for (parsed, source) in src {
        println!();
        println!("----- Source: {:?} -----", source.origin());
        match parsed {
            Ok(p) => println!("Parsed: {:?}", p),
            Err(e) => println!("Error: {}", e.with_source(source)),
        }
    }
}
