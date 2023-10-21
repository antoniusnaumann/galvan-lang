fn main() {
    print_parse_results();
}

fn print_parse_results() {
    use galvan_parser::{exec::parse_current_dir, ItemWithSource};

    let src = parse_current_dir();
    for (parsed, source) in src {
        println!();
        println!("----- Source: {:?} -----", source.origin().unwrap_or("?"));
        match parsed {
            Ok(p) => println!("Parsed: {:#?}", p),
            Err(e) => println!("Error: {}", e.with_source(source)),
        }
    }
}
