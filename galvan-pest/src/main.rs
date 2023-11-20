use galvan_pest::exec::parse_current_dir;

fn main() {
    let src = parse_current_dir();
    for (parsed, source) in &src {
        println!();
        println!("----- Source: {:?} -----", source.origin().unwrap_or("?"));
        match parsed {
            Ok(p) => println!("Parsed: {:#?}", p),
            Err(e) => println!("Error: {}", e),
        }
    }

    println!("\n----- Summary -----");
   println!("{} out of {} files parsed successfully", src.iter().filter(|(p, _)| p.is_ok()).count(), src.len());
}
