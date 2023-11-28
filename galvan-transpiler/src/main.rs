use std::env;

use galvan_transpiler::exec::transpile_dir;
use galvan_transpiler::Transpilation;

#[allow(clippy::redundant_closure)]
fn main() {
    let current_dir = env::current_dir().unwrap();
    let src = transpile_dir(current_dir);

    for Transpilation { transpiled, source } in src {
        println!();
        println!("----- Source: {:?} -----", source.origin());
        match transpiled {
            Ok(s) => println!("{}", s),
            Err(e) => println!("Error: {}", e),
        }
    }
}
