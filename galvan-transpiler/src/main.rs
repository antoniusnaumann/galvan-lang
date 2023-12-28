use std::env;

use galvan_transpiler::exec::transpile_dir;
use galvan_transpiler::Transpilation;

#[allow(clippy::redundant_closure)]
fn main() {
    let current_dir = env::current_dir().unwrap();
    let transpiled = transpile_dir(current_dir).unwrap();

    for output in transpiled {
        println!();
        println!("----- Output: {:?} -----", output.file_name);
        println!("{}", output.content);
    }
}
