use galvan_transpiler::transpile_source;
use std::env;
use walkdir::WalkDir;

use galvan_ast::{IntoAst, Source};

#[allow(clippy::redundant_closure)]
fn main() {
    let current_dir = env::current_dir().unwrap();

    let src = WalkDir::new(current_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension() == Some("galvan".as_ref()))
        .map(|p| Source::read(p))
        .map(|s| (transpile_source(s.clone()), s))
        .collect::<Vec<_>>();

    // TODO: Aggregate and print errors

    for (transpiled, source) in src {
        println!();
        println!("----- Source: {:?} -----", source.origin());
        match transpiled {
            Ok(s) => println!("{}", s),
            Err(e) => println!("Error: {}", e),
        }
    }
}
