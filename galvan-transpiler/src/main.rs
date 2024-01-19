use std::env;

use galvan_transpiler::exec::transpile_dir;

#[allow(clippy::redundant_closure)]
fn main() {
    let args: Vec<String> = env::args()
        .map(|s| {
            if !s.ends_with(".galvan") {
                s + ".galvan"
            } else {
                s
            }
        })
        .collect();

    let current_dir = env::current_dir().unwrap();
    let transpiled = transpile_dir(current_dir, args).unwrap();

    for output in transpiled {
        println!();
        println!("----- Output: {:?} -----", output.file_name);
        println!("{}", output.content);
    }
}
