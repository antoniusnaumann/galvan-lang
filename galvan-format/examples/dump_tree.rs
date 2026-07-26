use galvan_files::Source;

fn main() {
    let text = std::env::args().nth(1).unwrap();
    let tree = galvan_parse::parse_source(&Source::from_string(text.clone())).unwrap();
    println!("{}", tree.root_node().to_sexp());

    fn walk(node: galvan_parse::Node, depth: usize, src: &str) {
        let mut cursor = node.walk();
        println!(
            "{}{} [named={} children={}] {:?}",
            "  ".repeat(depth),
            node.kind(),
            node.is_named(),
            node.child_count(),
            &src[node.start_byte()..node.end_byte().min(node.start_byte() + 20)]
        );
        for child in node.children(&mut cursor) {
            walk(child, depth + 1, src);
        }
    }
    walk(tree.root_node(), 0, &text);
}
