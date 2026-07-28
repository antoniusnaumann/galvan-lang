use galvan_files::Source;
use galvan_transpiler::transpile;

#[test]
fn generates_default_impl_for_fully_defaulted_struct() {
    let outputs = transpile(vec![Source::from_string(
        "type Book {
             title: String = \"Field Notes\"
             pages: Int = 0
         }",
    )])
    .expect("source should transpile");
    let output = outputs
        .iter()
        .find(|output| output.file_name.as_ref() == "book.rs")
        .expect("book type file should be generated");

    assert!(output.content.contains("impl Default for Book"));
    assert!(output
        .content
        .contains("fn default() -> Self { Self { title: format!(\"Field Notes\"), pages: 0 } }"));
}

#[test]
fn skips_default_impl_for_partially_defaulted_struct() {
    let outputs = transpile(vec![Source::from_string(
        "type Book {
             title: String = \"Field Notes\"
             pages: Int
         }",
    )])
    .expect("source should transpile");
    let output = outputs
        .iter()
        .find(|output| output.file_name.as_ref() == "book.rs")
        .expect("book type file should be generated");

    assert!(!output.content.contains("impl Default for Book"));
}
