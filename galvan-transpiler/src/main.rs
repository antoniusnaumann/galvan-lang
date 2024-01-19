use itertools::Itertools;
use std::env;

use galvan_transpiler::exec::transpile_dir;

#[allow(clippy::redundant_closure)]
fn main() {
    let args: Vec<String> = env::args()
        .skip(1)
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            if !s.ends_with(".galvan") {
                s + ".galvan"
            } else {
                s
            }
        })
        .collect();

    let current_dir = env::current_dir().unwrap();
    println!("Args: {:?}", args);
    let transpiled = transpile_dir(current_dir, args).unwrap();

    for output in transpiled {
        println!();
        println!("----- Output: {:?} -----", output.file_name);
        println!("{}", output.content);
    }
}
