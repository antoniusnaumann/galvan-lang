//! End-to-end tests for the language-server features against real Galvan source.

use std::path::{Path, PathBuf};

use dashmap::DashMap;
use galvan_lsp::document::Document;
use galvan_lsp::features::{completion, diagnostics, goto_definition, hover, references};
use galvan_lsp::workspace::Crate;
use tower_lsp::lsp_types::{
    CompletionItemKind, DiagnosticSeverity, HoverContents, MarkupContent, Position, Url,
};

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

fn walk(self: Dog, distance: Int) {
}

fn main_fn() {
    let dog = Dog(name: \"Rex\")
    dog.walk(5)
    println(dog.name)
}
";

const MAIN_PATH: &str = "/galvan_lsp_test/src/main.galvan";

fn main_path() -> PathBuf {
    PathBuf::from(MAIN_PATH)
}

/// LSP position of the `nth` (0-based) occurrence of `needle` in `text`.
fn position_of(text: &str, needle: &str, nth: usize) -> Position {
    let byte = byte_of(text, needle, nth);
    Document::new(text).line_index.position(text, byte)
}

fn byte_of(text: &str, needle: &str, nth: usize) -> usize {
    text.match_indices(needle)
        .nth(nth)
        .unwrap_or_else(|| panic!("occurrence {nth} of {needle:?} not found"))
        .0
}

/// A single-file crate at a synthetic absolute path.
fn single_file_crate(text: &str) -> Crate {
    Crate::in_memory([(main_path(), text.into())])
}

fn hover_text(source: &str, position: Position) -> Option<String> {
    let doc = Document::new(source);
    let krate = single_file_crate(source);
    hover::hover(&doc, &krate, Some(&main_path()), position).map(|h| match h.contents {
        HoverContents::Markup(MarkupContent { value, .. }) => value,
        other => panic!("unexpected hover contents: {other:?}"),
    })
}

fn definition_of(source: &str, position: Position) -> Option<(Url, Position)> {
    let doc = Document::new(source);
    let krate = single_file_crate(source);
    goto_definition::goto_definition(&doc, &krate, Some(&main_path()), position)
        .map(|location| (location.uri, location.range.start))
}

/// Byte offset a definition points to within `source`.
fn definition_offset(source: &str, position: Position) -> usize {
    let (_, start) = definition_of(source, position).expect("expected a definition");
    Document::new(source)
        .line_index
        .offset(source, start)
        .unwrap()
}

// ----------------------------------------------------------------------
// Hover
// ----------------------------------------------------------------------

#[test]
fn hover_on_function_call_shows_signature() {
    // Second occurrence of `greet` is the call inside the function body.
    let text = hover_text(SOURCE, position_of(SOURCE, "greet", 1)).expect("expected hover");
    assert!(text.contains("fn greet"), "hover was: {text}");
    assert!(text.contains("name: String"), "hover was: {text}");
}

#[test]
fn hover_on_type_shows_declaration() {
    // The `Dog` in `self: Dog`.
    let text = hover_text(SOURCE, position_of(SOURCE, "Dog", 1)).expect("expected hover");
    assert!(text.contains("type Dog"), "hover was: {text}");
}

#[test]
fn hover_on_local_variable_shows_its_type() {
    // The `dog` in `dog.walk(5)`.
    let text = hover_text(SOURCE, position_of(SOURCE, "dog", 1)).expect("expected hover");
    assert!(text.contains("let dog: Dog"), "hover was: {text}");
}

#[test]
fn hover_on_parameter_shows_its_type() {
    // The `name` used in the call `greet(name)`.
    let text = hover_text(SOURCE, position_of(SOURCE, "name", 1)).expect("expected hover");
    assert!(text.contains("name: String"), "hover was: {text}");
}

#[test]
fn hover_on_field_access_shows_field() {
    // The `name` in `self.name`.
    let text = hover_text(SOURCE, position_of(SOURCE, "name", 3)).expect("expected hover");
    assert!(text.contains("Dog.name: String"), "hover was: {text}");
}

#[test]
fn hover_on_method_call_shows_signature() {
    // The `walk` in `dog.walk(5)`.
    let text = hover_text(SOURCE, position_of(SOURCE, "walk", 1)).expect("expected hover");
    assert!(text.contains("fn walk(self: Dog"), "hover was: {text}");
}

// ----------------------------------------------------------------------
// Go to definition
// ----------------------------------------------------------------------

#[test]
fn goto_definition_jumps_to_function_name() {
    let offset = definition_offset(SOURCE, position_of(SOURCE, "greet", 1));
    // The target is the identifier of the declaration on the first line.
    assert_eq!(offset, byte_of(SOURCE, "greet", 0));
}

#[test]
fn goto_definition_jumps_to_type_declaration() {
    let offset = definition_offset(SOURCE, position_of(SOURCE, "Dog", 1));
    assert_eq!(offset, byte_of(SOURCE, "Dog", 0));
}

#[test]
fn goto_definition_resolves_local_variables() {
    // From the use in `println(dog.name)` to the `let dog` declaration.
    let offset = definition_offset(SOURCE, position_of(SOURCE, "dog", 2));
    assert_eq!(offset, byte_of(SOURCE, "dog", 0));
}

#[test]
fn goto_definition_resolves_method_calls() {
    // From `dog.walk(5)` to `fn walk(self: Dog, ...)`.
    let offset = definition_offset(SOURCE, position_of(SOURCE, "walk", 1));
    assert_eq!(offset, byte_of(SOURCE, "walk", 0));
}

#[test]
fn goto_definition_resolves_fields() {
    // From `self.name` to the field declaration inside `type Dog`.
    let offset = definition_offset(SOURCE, position_of(SOURCE, "name", 3));
    assert_eq!(offset, byte_of(SOURCE, "name", 2));
}

#[test]
fn goto_definition_resolves_parameters() {
    // From `greet(name)` to the parameter `name` of `fn greet`.
    let offset = definition_offset(SOURCE, position_of(SOURCE, "name", 1));
    assert_eq!(offset, byte_of(SOURCE, "name", 0));
}

// ----------------------------------------------------------------------
// Find references
// ----------------------------------------------------------------------

fn reference_offsets(source: &str, position: Position, include_declaration: bool) -> Vec<usize> {
    let doc = Document::new(source);
    let krate = single_file_crate(source);
    let index = Document::new(source);
    let mut offsets: Vec<usize> = references::references(
        &doc,
        &krate,
        Some(&main_path()),
        position,
        include_declaration,
    )
    .into_iter()
    .map(|location| {
        index
            .line_index
            .offset(source, location.range.start)
            .unwrap()
    })
    .collect();
    offsets.sort();
    offsets
}

#[test]
fn references_on_local_variable_finds_all_uses() {
    let offsets = reference_offsets(SOURCE, position_of(SOURCE, "dog", 1), true);
    assert_eq!(
        offsets,
        vec![
            byte_of(SOURCE, "dog", 0),
            byte_of(SOURCE, "dog", 1),
            byte_of(SOURCE, "dog", 2),
        ]
    );
}

#[test]
fn references_on_type_finds_annotations_and_constructors() {
    let offsets = reference_offsets(SOURCE, position_of(SOURCE, "Dog", 0), true);
    // Declaration + `self: Dog` (twice) + constructor `Dog(name: ...)`.
    assert_eq!(offsets.len(), 4, "references were at offsets: {offsets:?}");
    assert!(offsets.contains(&byte_of(SOURCE, "Dog", 0)));
    assert!(offsets.contains(&byte_of(SOURCE, "Dog(name:", 0)));
}

#[test]
fn references_on_function_finds_calls() {
    let offsets = reference_offsets(SOURCE, position_of(SOURCE, "greet", 0), false);
    assert_eq!(
        offsets,
        vec![byte_of(SOURCE, "greet", 1), byte_of(SOURCE, "greet", 2)]
    );
}

// ----------------------------------------------------------------------
// Completion
// ----------------------------------------------------------------------

fn completion_labels(source: &str, position: Position) -> Vec<(String, Option<CompletionItemKind>)> {
    let doc = Document::new(source);
    let krate = single_file_crate(source);
    completion::completion(&doc, &krate, Some(&main_path()), position)
        .into_iter()
        .map(|item| (item.label, item.kind))
        .collect()
}

#[test]
fn completion_includes_declarations_and_keywords() {
    // Inside the body of `main_fn`, after the declaration of `dog`.
    let position = position_of(SOURCE, "dog.walk", 0);
    let labels = completion_labels(SOURCE, position);

    let names: Vec<&str> = labels.iter().map(|(l, _)| l.as_str()).collect();
    assert!(names.contains(&"greet"));
    assert!(names.contains(&"Dog"));
    assert!(names.contains(&"fn"));
    assert!(names.contains(&"type"));
}

#[test]
fn completion_offers_locals_in_scope() {
    let position = position_of(SOURCE, "dog.walk", 0);
    let labels = completion_labels(SOURCE, position);
    assert!(
        labels
            .iter()
            .any(|(l, k)| l == "dog" && *k == Some(CompletionItemKind::VARIABLE)),
        "labels were: {labels:?}"
    );
}

#[test]
fn completion_does_not_offer_out_of_scope_locals() {
    // Inside `greet`, the local `dog` of `main_fn` is not in scope.
    let position = position_of(SOURCE, "greet(name)", 0);
    let labels = completion_labels(SOURCE, position);
    assert!(
        !labels.iter().any(|(l, _)| l == "dog"),
        "labels were: {labels:?}"
    );
    // But greet's own parameter is.
    assert!(
        labels.iter().any(|(l, _)| l == "name"),
        "labels were: {labels:?}"
    );
}

#[test]
fn completion_after_dot_offers_fields_and_methods() {
    // Insert a fresh `dog.` right after the walk call and complete there.
    let call_end = byte_of(SOURCE, "dog.walk(5)", 0) + "dog.walk(5)".len();
    let mut source = SOURCE.to_string();
    source.insert_str(call_end, "\n    dog.");
    let offset = call_end + "\n    dog.".len();
    let doc = Document::new(source.as_str());
    let position = doc.line_index.position(&source, offset);

    let labels = completion_labels(&source, position);
    assert!(
        labels
            .iter()
            .any(|(l, k)| l == "name" && *k == Some(CompletionItemKind::FIELD)),
        "labels were: {labels:?}"
    );
    assert!(
        labels
            .iter()
            .any(|(l, k)| l == "walk" && *k == Some(CompletionItemKind::METHOD)),
        "labels were: {labels:?}"
    );
    // Unrelated top-level names are not offered after a dot.
    assert!(
        !labels.iter().any(|(l, _)| l == "greet"),
        "labels were: {labels:?}"
    );
}

// ----------------------------------------------------------------------
// Cross-file resolution
// ----------------------------------------------------------------------

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

    let location = goto_definition::goto_definition(
        &doc,
        &krate,
        Some(Path::new("/galvan_lsp_test/src/a.galvan")),
        position_of(a_src, "greet", 0),
    )
    .expect("cross-file definition");

    // The definition lives in b.galvan, not the requesting file.
    assert_eq!(location.uri, Url::from_file_path(&b_path).unwrap());
    let start = Document::new(b_src)
        .line_index
        .offset(b_src, location.range.start)
        .unwrap();
    assert!(b_src[start..].starts_with("greet"));
}

#[test]
fn references_resolve_across_files_in_the_same_crate() {
    let a_path = PathBuf::from("/galvan_lsp_test/src/a.galvan");
    let b_path = PathBuf::from("/galvan_lsp_test/src/b.galvan");
    let a_src = "fn caller() {\n    shared()\n}\n";
    let b_src = "fn shared() {}\nfn other() {\n    shared()\n}\n";

    let krate = Crate::in_memory([
        (a_path.clone(), a_src.to_string()),
        (b_path.clone(), b_src.to_string()),
    ]);
    let doc = Document::new(b_src);

    let locations = references::references(
        &doc,
        &krate,
        Some(&b_path),
        position_of(b_src, "shared", 0),
        false,
    );
    let uris: Vec<&Url> = locations.iter().map(|location| &location.uri).collect();
    assert_eq!(locations.len(), 2, "locations: {locations:?}");
    assert!(uris.contains(&&Url::from_file_path(&a_path).unwrap()));
    assert!(uris.contains(&&Url::from_file_path(&b_path).unwrap()));
}

#[test]
fn completion_aggregates_symbols_from_all_crate_files() {
    let a_path = PathBuf::from("/galvan_lsp_test/src/a.galvan");
    let krate = Crate::in_memory([
        (a_path.clone(), "fn alpha() {\n    beta()\n}\n".to_string()),
        (
            PathBuf::from("/galvan_lsp_test/src/b.galvan"),
            "fn beta() {}\ntype Gamma {}\n".to_string(),
        ),
    ]);
    let a_src = "fn alpha() {\n    beta()\n}\n";
    let doc = Document::new(a_src);

    let labels: Vec<String> = completion::completion(
        &doc,
        &krate,
        Some(&a_path),
        position_of(a_src, "beta", 0),
    )
    .into_iter()
    .map(|item| item.label)
    .collect();

    assert!(labels.iter().any(|l| l == "alpha"));
    assert!(labels.iter().any(|l| l == "beta"));
    assert!(labels.iter().any(|l| l == "Gamma"));
}

// ----------------------------------------------------------------------
// Diagnostics
// ----------------------------------------------------------------------

#[test]
fn semantic_diagnostics_report_type_errors_with_a_range() {
    let path = main_path();
    // Referencing an undefined identifier is a semantic error.
    let src = "fn f() {\n    print(undefined_variable)\n}\n";
    let krate = Crate::in_memory([(path.clone(), src.to_string())]);
    let doc = Document::new(src);

    let diags = diagnostics::diagnostics(&doc, &krate, Some(&path));
    let semantic: Vec<_> = diags
        .iter()
        .filter(|d| {
            d.severity == Some(DiagnosticSeverity::ERROR)
                && d.message.contains("Unknown identifier")
        })
        .collect();

    assert!(
        !semantic.is_empty(),
        "expected an unknown-identifier error, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    // The range points at the offending reference (line 1, 0-based).
    let diag = semantic[0];
    assert_eq!(diag.range.start.line, 1, "diagnostic range: {:?}", diag.range);
}

#[test]
fn duplicate_declarations_are_reported_not_fatal() {
    let path = main_path();
    let src = "type Dup {}\ntype Dup {}\nfn f() {}\nfn f() {}\n";
    let krate = Crate::in_memory([(path.clone(), src.to_string())]);
    let doc = Document::new(src);

    let diags = diagnostics::diagnostics(&doc, &krate, Some(&path));
    let duplicates: Vec<_> = diags
        .iter()
        .filter(|d| d.message.contains("Duplicate"))
        .collect();
    assert_eq!(
        duplicates.len(),
        2,
        "expected duplicate type and function errors, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    // The duplicate *type* diagnostic points at the second declaration.
    assert!(
        duplicates.iter().any(|d| d.range.start.line == 1),
        "diagnostics: {duplicates:?}"
    );
}

#[test]
fn clean_program_has_no_semantic_diagnostics() {
    let path = main_path();
    let src = "fn add(a: Int, b: Int) -> Int {\n    a + b\n}\n";
    let krate = Crate::in_memory([(path.clone(), src.to_string())]);
    let doc = Document::new(src);

    let diags = diagnostics::diagnostics(&doc, &krate, Some(&path));
    assert!(
        diags.is_empty(),
        "expected no diagnostics, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
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

    let location = goto_definition::goto_definition(
        &doc,
        &krate,
        Some(&a_path),
        position_of(open_text, "helper", 0),
    )
    .expect("definition resolved from a sibling file on disk");
    assert_eq!(location.uri, Url::from_file_path(&b_path).unwrap());

    std::fs::remove_dir_all(&root).ok();
}
