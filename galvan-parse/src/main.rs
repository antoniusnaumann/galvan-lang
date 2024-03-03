use galvan_parse::exec::parse_current_dir;
use std::path::Path;
use tree_sitter::{Node, Tree};

fn main() {
    println!("Parsing...");
    let src = parse_current_dir();
    println!("Parsing finished!");
    for (parsed, source) in &src {
        println!();
        println!(
            "----- Source: {:?} -----",
            source
                .origin()
                .map(Path::to_string_lossy)
                .unwrap_or("?".into())
        );
        match parsed {
            Ok(p) => print_tree(p),
            Err(e) => println!("Error: {}", e),
        }
    }

    println!("\n----- Summary -----");
    println!(
        "{} out of {} files parsed successfully",
        src.iter().filter(|(p, _)| p.is_ok()).count(),
        src.len()
    );
}

fn print_tree(t: &Tree) {
    println!("Parsed:");
    print_node(t.root_node(), 0);
}

fn print_node(n: Node, indent: usize) {
    let pre = "  ".repeat(indent);
    println!("{pre}{}", n.kind());

    let mut cursor = n.walk();
    for child in n.children(&mut cursor) {
        if child.is_named() {
            print_node(child, indent + 1);
        }
    }
}
