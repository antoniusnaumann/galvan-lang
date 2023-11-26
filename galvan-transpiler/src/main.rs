use std::env;

use galvan_transpiler::RustSource;
use galvan_transpiler::exec::transpile_dir;

#[allow(clippy::redundant_closure)]
fn main() {
    let current_dir = env::current_dir().unwrap();
    let src = transpile_dir(current_dir);

    for RustSource { transpiled, source } in src {
        println!();
        println!("----- Source: {:?} -----", source.origin());
        match transpiled {
            Ok(s) => println!("{}", s),
            Err(e) => println!("Error: {}", e),
        }
    }
}
