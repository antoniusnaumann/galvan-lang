use galvan_transpiler::transpile_source;
use std::env;
use walkdir::WalkDir;

use galvan_ast::Source;

#[allow(clippy::redundant_closure)]
fn main() {
    let current_dir = env::current_dir().unwrap();

    let src = WalkDir::new(current_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension() == Some("galvan".as_ref()))
        .map(|p| Source::read(p))
        .map(|s| (parse_source(&s), s))
        .collect::<Vec<_>>();

    // TODO: Aggregate and print errors

    for (parsed, source) in src {
        println!();
        println!("----- Source: {:?} -----", source.origin());
        match parsed {
            Ok(p) => println!("{}", transpile_source(p)),
            Err(e) => println!("Error: {}", e.with_source(source)),
        }
    }
}
