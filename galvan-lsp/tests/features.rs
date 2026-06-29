//! End-to-end tests for the language-server features against real Galvan source.

use std::path::PathBuf;

use dashmap::DashMap;
use galvan_lsp::document::Document;
use galvan_lsp::features::{completion, goto_definition, hover};
use galvan_lsp::workspace::Crate;
use tower_lsp::lsp_types::{HoverContents, MarkupContent, Position, Url};

const SOURCE: &str = "\
fn greet(name: String) {
    greet(name)
}

type Dog {
    name: String
}

fn pet(self: Dog) {
    greet(self.name)
}
";

/// LSP position of the `nth` (0-based) occurrence of `needle` in `text`.
fn position_of(text: &str, needle: &str, nth: usize) -> Position {
    let byte = text
        .match_indices(needle)
        .nth(nth)
        .unwrap_or_else(|| panic!("occurrence {nth} of {needle:?} not found"))
        .0;
    Document::new(text).line_index.position(text, byte)
}

/// A single-file crate at a synthetic absolute path.
fn single_file_crate(text: &str) -> Crate {
    Crate::in_memory([(PathBuf::from("/galvan_lsp_test/src/main.galvan"), text.into())])
}

fn hover_text(position: Position) -> Option<String> {
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    hover::hover(&doc, &krate, position).map(|h| match h.contents {
        HoverContents::Markup(MarkupContent { value, .. }) => value,
        other => panic!("unexpected hover contents: {other:?}"),
    })
}

#[test]
fn hover_on_function_call_shows_signature() {
    // Second occurrence of `greet` is the call inside the function body.
    let text = hover_text(position_of(SOURCE, "greet", 1)).expect("expected hover");
    assert!(text.contains("fn greet"), "hover was: {text}");
    assert!(text.contains("name: String"), "hover was: {text}");
}

#[test]
fn hover_on_type_shows_declaration() {
    // The `Dog` in `self: Dog`.
    let text = hover_text(position_of(SOURCE, "Dog", 1)).expect("expected hover");
    assert!(text.contains("type Dog"), "hover was: {text}");
}

#[test]
fn goto_definition_jumps_to_function_signature() {
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let location = goto_definition::goto_definition(&doc, &krate, position_of(SOURCE, "greet", 1))
        .expect("definition");

    assert_eq!(location.range.start.line, 0);
    let target = Document::new(SOURCE);
    let start = target.line_index.offset(SOURCE, location.range.start).unwrap();
    assert!(SOURCE[start..].starts_with("fn greet"));
}

#[test]
fn goto_definition_jumps_to_type_declaration() {
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let location = goto_definition::goto_definition(&doc, &krate, position_of(SOURCE, "Dog", 1))
        .expect("definition");

    let target = Document::new(SOURCE);
    let start = target.line_index.offset(SOURCE, location.range.start).unwrap();
    assert!(SOURCE[start..].starts_with("type Dog"));
}

#[test]
fn completion_includes_declarations_and_keywords() {
    let krate = single_file_crate(SOURCE);
    let labels: Vec<String> = completion::completion(&krate)
        .into_iter()
        .map(|item| item.label)
        .collect();

    assert!(labels.iter().any(|l| l == "greet"));
    assert!(labels.iter().any(|l| l == "pet"));
    assert!(labels.iter().any(|l| l == "Dog"));
    assert!(labels.iter().any(|l| l == "fn"));
    assert!(labels.iter().any(|l| l == "type"));
}

#[test]
fn goto_definition_resolves_across_files_in_the_same_crate() {
    let a_path = PathBuf::from("/galvan_lsp_test/src/a.galvan");
    let b_path = PathBuf::from("/galvan_lsp_test/src/b.galvan");
    let a_src = "fn caller() {\n    greet()\n}\n";
    let b_src = "fn greet() {\n    println(\"hi\")\n}\n";

    let krate = Crate::in_memory([
        (a_path.clone(), a_src.to_string()),
        (b_path.clone(), b_src.to_string()),
    ]);
    let doc = Document::new(a_src);

    let location = goto_definition::goto_definition(&doc, &krate, position_of(a_src, "greet", 0))
        .expect("cross-file definition");

    // The definition lives in b.galvan, not the requesting file.
    assert_eq!(location.uri, Url::from_file_path(&b_path).unwrap());
    let start = Document::new(b_src)
        .line_index
        .offset(b_src, location.range.start)
        .unwrap();
    assert!(b_src[start..].starts_with("fn greet"));
}

#[test]
fn completion_aggregates_symbols_from_all_crate_files() {
    let krate = Crate::in_memory([
        (
            PathBuf::from("/galvan_lsp_test/src/a.galvan"),
            "fn alpha() {}\n".to_string(),
        ),
        (
            PathBuf::from("/galvan_lsp_test/src/b.galvan"),
            "fn beta() {}\ntype Gamma {}\n".to_string(),
        ),
    ]);

    let labels: Vec<String> = completion::completion(&krate)
        .into_iter()
        .map(|item| item.label)
        .collect();

    assert!(labels.iter().any(|l| l == "alpha"));
    assert!(labels.iter().any(|l| l == "beta"));
    assert!(labels.iter().any(|l| l == "Gamma"));
}

/// Exercise the real on-disk loader: a crate laid out as `<tmp>/src/*.galvan`,
/// with the requesting file open in the editor and the definition on disk.
#[test]
fn load_reads_crate_files_from_disk_with_open_overrides() {
    let root = std::env::temp_dir().join(format!("galvan_lsp_load_{}", std::process::id()));
    let src = root.join("src");
    std::fs::create_dir_all(&src).unwrap();

    let a_path = src.join("a.galvan");
    let b_path = src.join("b.galvan");
    // `a` exists on disk but is also open with an unsaved call to `helper`.
    std::fs::write(&a_path, "fn caller() {}\n").unwrap();
    std::fs::write(&b_path, "fn helper() {}\n").unwrap();

    let a_uri = Url::from_file_path(&a_path).unwrap();
    let open = DashMap::new();
    let open_text = "fn caller() {\n    helper()\n}\n";
    open.insert(a_uri.clone(), Document::new(open_text));

    let krate = Crate::load(&a_uri, &open);
    let doc = Document::new(open_text);

    let location =
        goto_definition::goto_definition(&doc, &krate, position_of(open_text, "helper", 0))
            .expect("definition resolved from a sibling file on disk");
    assert_eq!(location.uri, Url::from_file_path(&b_path).unwrap());

    std::fs::remove_dir_all(&root).ok();
}
