//! End-to-end tests for the language-server features against real Galvan source.

use galvan_lsp::document::Document;
use galvan_lsp::features::{completion, goto_definition, hover};
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

/// LSP position of the `nth` (0-based) occurrence of `needle` in `SOURCE`.
fn position_of(needle: &str, nth: usize) -> Position {
    let byte = SOURCE
        .match_indices(needle)
        .nth(nth)
        .unwrap_or_else(|| panic!("occurrence {nth} of {needle:?} not found"))
        .0;
    let doc = Document::new(SOURCE);
    doc.line_index.position(SOURCE, byte)
}

fn hover_text(position: Position) -> Option<String> {
    let doc = Document::new(SOURCE);
    hover::hover(&doc, position).map(|h| match h.contents {
        HoverContents::Markup(MarkupContent { value, .. }) => value,
        other => panic!("unexpected hover contents: {other:?}"),
    })
}

#[test]
fn hover_on_function_call_shows_signature() {
    // Second occurrence of `greet` is the call inside the function body.
    let text = hover_text(position_of("greet", 1)).expect("expected hover");
    assert!(text.contains("fn greet"), "hover was: {text}");
    assert!(text.contains("name: String"), "hover was: {text}");
}

#[test]
fn hover_on_type_shows_declaration() {
    // The `Dog` in `self: Dog`.
    let text = hover_text(position_of("Dog", 1)).expect("expected hover");
    assert!(text.contains("type Dog"), "hover was: {text}");
}

#[test]
fn goto_definition_jumps_to_function_signature() {
    let uri = Url::parse("file:///test.galvan").unwrap();
    let doc = Document::new(SOURCE);
    let location =
        goto_definition::goto_definition(&doc, uri, position_of("greet", 1)).expect("definition");

    // The definition is the first line of the file.
    assert_eq!(location.range.start.line, 0);
    let start = doc
        .line_index
        .offset(SOURCE, location.range.start)
        .unwrap();
    assert!(SOURCE[start..].starts_with("fn greet"));
}

#[test]
fn goto_definition_jumps_to_type_declaration() {
    let uri = Url::parse("file:///test.galvan").unwrap();
    let doc = Document::new(SOURCE);
    let location =
        goto_definition::goto_definition(&doc, uri, position_of("Dog", 1)).expect("definition");

    let start = doc
        .line_index
        .offset(SOURCE, location.range.start)
        .unwrap();
    assert!(SOURCE[start..].starts_with("type Dog"));
}

#[test]
fn completion_includes_declarations_and_keywords() {
    let doc = Document::new(SOURCE);
    let labels: Vec<String> = completion::completion(&doc)
        .into_iter()
        .map(|item| item.label)
        .collect();

    assert!(labels.iter().any(|l| l == "greet"));
    assert!(labels.iter().any(|l| l == "pet"));
    assert!(labels.iter().any(|l| l == "Dog"));
    assert!(labels.iter().any(|l| l == "fn"));
    assert!(labels.iter().any(|l| l == "type"));
}
