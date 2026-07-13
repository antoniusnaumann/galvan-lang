//! End-to-end tests for the language-server features against real Galvan source.

use std::path::{Path, PathBuf};

use dashmap::DashMap;
use galvan_lsp::document::Document;
use galvan_lsp::features::{
    code_actions, completion, diagnostics, formatting, goto_definition, hover, inlay_hints,
    references, rename, semantic_tokens, signature_help, symbols,
};
use galvan_lsp::workspace::Crate;
use tower_lsp::lsp_types::{
    CompletionItemKind, DiagnosticSeverity, HoverContents, MarkupContent, ParameterLabel,
    Position, SignatureHelp, Url,
};

const SOURCE: &str = "\
fn greet(name: String) {
    greet(name)
}

type Dog {
    name: String,
}

fn pet(self: Dog) {
    greet(self.name)
}

fn walk(self: Dog, distance: Int) {}

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

#[test]
fn hover_at_end_of_identifier_still_resolves() {
    // Cursor directly BEHIND `dog` in `dog.walk(5)` — the position right
    // after typing the name.
    let mut position = position_of(SOURCE, "dog.walk", 0);
    position.character += "dog".len() as u32;
    let text = hover_text(SOURCE, position).expect("expected hover at end of identifier");
    assert!(text.contains("let dog: Dog"), "hover was: {text}");
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
    // Statement keywords are offered at statement start...
    assert!(names.contains(&"let"));
    assert!(names.contains(&"return"));
    // ...but declaration keywords only make sense at the top level.
    assert!(!names.contains(&"fn"));
    assert!(!names.contains(&"type"));
}

#[test]
fn completion_aliases_foreign_keywords_in_statement_position() {
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let position = position_of(SOURCE, "dog.walk", 0);
    let items = completion::completion(&doc, &krate, Some(&main_path()), position);

    // Typing `switch` surfaces `match`: the alias filters, the label inserts.
    let alias = items
        .iter()
        .find(|item| item.filter_text.as_deref() == Some("switch"))
        .expect("alias for `switch` offered");
    assert_eq!(alias.label, "match");
    assert_eq!(alias.kind, Some(CompletionItemKind::KEYWORD));

    let var_alias = items
        .iter()
        .find(|item| item.filter_text.as_deref() == Some("var"))
        .expect("alias for `var` offered");
    assert_eq!(var_alias.label, "mut");

    // `fn` is a top-level keyword; its aliases make no sense mid-body.
    assert!(
        !items
            .iter()
            .any(|item| item.filter_text.as_deref() == Some("func")),
        "no `func` alias inside a body"
    );
}

#[test]
fn completion_aliases_foreign_keywords_at_top_level() {
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    // On the `fn` of a top-level declaration: only declarations can follow.
    let position = position_of(SOURCE, "fn main_fn", 0);
    let items = completion::completion(&doc, &krate, Some(&main_path()), position);

    for (foreign, galvan) in [("func", "fn"), ("class", "type"), ("import", "use")] {
        let alias = items
            .iter()
            .find(|item| item.filter_text.as_deref() == Some(foreign))
            .unwrap_or_else(|| panic!("alias for `{foreign}` offered"));
        assert_eq!(alias.label, galvan);
    }
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

const ENUM_SOURCE: &str = "\
type Color {
    Transparent
    Gray(U8)
    Rgb(r: U8, g: U8, b: U8)
}

fn describe(color: Color) {
    println(color)
}

fn main_fn() {
    let color = Color::Transparent
    describe(color)
}
";

#[test]
fn completion_after_enum_path_offers_only_variants() {
    // Complete on the `Transparent` in `Color::Transparent`.
    let position = position_of(ENUM_SOURCE, "Transparent", 1);
    let labels = completion_labels(ENUM_SOURCE, position);

    for variant in ["Transparent", "Gray", "Rgb"] {
        assert!(
            labels
                .iter()
                .any(|(l, k)| l == variant && *k == Some(CompletionItemKind::ENUM_MEMBER)),
            "missing variant {variant}, labels were: {labels:?}"
        );
    }
    // No functions, types or keywords after `::`.
    assert_eq!(labels.len(), 3, "labels were: {labels:?}");
}

#[test]
fn completion_directly_after_dangling_enum_path() {
    // Add an incomplete `Color::` and complete right behind it.
    let mut source = ENUM_SOURCE.to_string();
    let insert_at = byte_of(ENUM_SOURCE, "    describe(color)", 0);
    source.insert_str(insert_at, "    let other = Color::\n");
    let offset = insert_at + "    let other = Color::".len();
    let doc = Document::new(source.as_str());
    let position = doc.line_index.position(&source, offset);

    let labels = completion_labels(&source, position);
    assert!(
        labels
            .iter()
            .any(|(l, k)| l == "Gray" && *k == Some(CompletionItemKind::ENUM_MEMBER)),
        "labels were: {labels:?}"
    );
    assert!(
        !labels.iter().any(|(l, _)| l == "describe"),
        "labels were: {labels:?}"
    );
}

#[test]
fn completion_after_unknown_path_qualifier_offers_nothing() {
    // `color::` (a value, not an enum) resolves to no variants — better
    // nothing than unrelated names.
    let source = ENUM_SOURCE.replace("describe(color)", "describe(color::x)");
    let position = position_of(&source, "x)", 0);
    let labels = completion_labels(&source, position);
    assert!(labels.is_empty(), "labels were: {labels:?}");
}

#[test]
fn completion_in_type_position_offers_only_types() {
    // Complete on the `String` in the parameter `name: String`.
    let position = position_of(SOURCE, "String", 0);
    let labels = completion_labels(SOURCE, position);

    assert!(
        labels
            .iter()
            .any(|(l, k)| l == "Dog" && *k == Some(CompletionItemKind::STRUCT)),
        "labels were: {labels:?}"
    );
    // Builtin types are offered too.
    assert!(
        labels.iter().any(|(l, _)| l == "Int"),
        "labels were: {labels:?}"
    );
    // No functions or keywords where a type is expected.
    assert!(
        !labels.iter().any(|(l, _)| l == "greet" || l == "fn" || l == "let"),
        "labels were: {labels:?}"
    );
}

#[test]
fn completion_in_field_type_position_offers_only_types() {
    // Complete on the `String` in the field `name: String` of `type Dog`.
    let position = position_of(SOURCE, "String", 1);
    let labels = completion_labels(SOURCE, position);

    assert!(
        labels.iter().any(|(l, _)| l == "Dog"),
        "labels were: {labels:?}"
    );
    assert!(
        !labels.iter().any(|(l, _)| l == "greet" || l == "pet"),
        "labels were: {labels:?}"
    );
}

#[test]
fn completion_after_constructor_arg_label_offers_values_not_types_only() {
    // The `:` in `Dog(name: "Rex")` labels an argument, so an expression is
    // expected: functions are offered again.
    let position = position_of(SOURCE, "\"Rex\"", 0);
    let labels = completion_labels(SOURCE, position);
    assert!(
        labels
            .iter()
            .any(|(l, k)| l == "greet" && *k == Some(CompletionItemKind::FUNCTION)),
        "labels were: {labels:?}"
    );
}

#[test]
fn completion_offers_nothing_for_new_names() {
    // Right on the fresh binding name in `let dog = ...`.
    let position = position_of(SOURCE, "dog", 0);
    let labels = completion_labels(SOURCE, position);
    assert!(labels.is_empty(), "labels were: {labels:?}");
}

#[test]
fn completion_at_toplevel_offers_declaration_keywords_only() {
    // On the `type` keyword of `type Dog`, at the top level of the file.
    let position = position_of(SOURCE, "type Dog", 0);
    let labels = completion_labels(SOURCE, position);

    let names: Vec<&str> = labels.iter().map(|(l, _)| l.as_str()).collect();
    assert!(names.contains(&"fn"));
    assert!(names.contains(&"type"));
    assert!(!names.contains(&"greet"), "labels were: {labels:?}");
    assert!(!names.contains(&"let"), "labels were: {labels:?}");
}

#[test]
fn completion_keywords_match_the_grammar() {
    // At statement start inside a body.
    let position = position_of(SOURCE, "dog.walk", 0);
    let labels = completion_labels(SOURCE, position);
    let names: Vec<&str> = labels.iter().map(|(l, _)| l.as_str()).collect();

    // Contextual statement starters and declaration modifiers are offered...
    for keyword in ["loop", "throw", "move", "none"] {
        assert!(names.contains(&keyword), "missing {keyword}: {names:?}");
    }
    // ...built-in statement functions too...
    assert!(names.contains(&"println"), "labels were: {names:?}");
    // ...but words that are not part of the language are not.
    for absent in ["async", "const", "main", "struct", "enum"] {
        assert!(!names.contains(&absent), "unexpected {absent}: {names:?}");
    }
}

#[test]
fn completion_dedupes_shadowed_locals() {
    let src = "fn f() {\n    let x = 1\n    let x = 2\n    println(x)\n}\n";
    let position = position_of(src, "x)", 0);
    let labels = completion_labels(src, position);
    let count = labels.iter().filter(|(l, _)| l == "x").count();
    assert_eq!(count, 1, "labels were: {labels:?}");
}

#[test]
fn completion_ranks_locals_before_types_and_keywords() {
    let position = position_of(SOURCE, "dog.walk", 0);
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let items = completion::completion(&doc, &krate, Some(&main_path()), position);

    let sort_key = |label: &str| {
        items
            .iter()
            .find(|item| item.label == label)
            .and_then(|item| item.sort_text.clone())
            .unwrap_or_else(|| panic!("no completion for {label}"))
    };
    assert!(sort_key("dog") < sort_key("greet"), "local before function");
    assert!(sort_key("greet") < sort_key("Dog"), "function before type");
    assert!(sort_key("Dog") < sort_key("let"), "type before keyword");
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
fn member_completion_survives_parse_error_when_siblings_parse() {
    // The current file has a dangling `dog.` (a parse error), but the crate
    // still analyzes because b.galvan parses — the probe must run anyway.
    let a_path = PathBuf::from("/galvan_lsp_test/src/a.galvan");
    let a_src = "fn f(dog: Dog) {\n    dog.\n}\n";
    let b_src = "type Dog {\n    name: String\n}\n\nfn walk(self: Dog) {\n}\n";
    let krate = Crate::in_memory([
        (a_path.clone(), a_src.to_string()),
        (
            PathBuf::from("/galvan_lsp_test/src/b.galvan"),
            b_src.to_string(),
        ),
    ]);
    let doc = Document::new(a_src);
    let offset = byte_of(a_src, "dog.", 0) + "dog.".len();
    let position = doc.line_index.position(a_src, offset);

    let labels: Vec<(String, Option<CompletionItemKind>)> =
        completion::completion(&doc, &krate, Some(&a_path), position)
            .into_iter()
            .map(|item| (item.label, item.kind))
            .collect();
    assert!(
        labels
            .iter()
            .any(|(l, k)| l == "walk" && *k == Some(CompletionItemKind::METHOD)),
        "labels were: {labels:?}"
    );
}

#[test]
fn hover_and_goto_fall_back_when_current_file_has_parse_error() {
    // a.galvan contains a parse error further down; greet lives in b.galvan.
    let a_path = PathBuf::from("/galvan_lsp_test/src/a.galvan");
    let a_src = "fn caller() {\n    greet()\n}\n\nfn broken() {\n    let x = \n}\n";
    let b_src = "fn greet() {\n}\n";
    let krate = Crate::in_memory([
        (a_path.clone(), a_src.to_string()),
        (
            PathBuf::from("/galvan_lsp_test/src/b.galvan"),
            b_src.to_string(),
        ),
    ]);
    let doc = Document::new(a_src);
    let position = position_of(a_src, "greet", 0);

    let hover = hover::hover(&doc, &krate, Some(&a_path), position);
    assert!(
        hover.is_some(),
        "expected name-based hover despite the parse error"
    );

    let location = goto_definition::goto_definition(&doc, &krate, Some(&a_path), position);
    assert!(
        location.is_some(),
        "expected name-based goto despite the parse error"
    );
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
// Rename
// ----------------------------------------------------------------------

#[test]
fn prepare_rename_returns_range_and_placeholder() {
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let response = rename::prepare_rename(
        &doc,
        &krate,
        Some(&main_path()),
        position_of(SOURCE, "dog", 1),
    )
    .expect("expected a renamable symbol");

    match response {
        tower_lsp::lsp_types::PrepareRenameResponse::RangeWithPlaceholder {
            placeholder, ..
        } => assert_eq!(placeholder, "dog"),
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn rename_local_variable_edits_every_use() {
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let edit = rename::rename(
        &doc,
        &krate,
        Some(&main_path()),
        position_of(SOURCE, "dog", 1),
        "hound",
    )
    .expect("expected a workspace edit");

    let changes = edit.changes.expect("expected changes");
    let uri = Url::from_file_path(main_path()).unwrap();
    let edits = changes.get(&uri).expect("edits for the main file");
    // Declaration + `dog.walk(5)` + `println(dog.name)`.
    assert_eq!(edits.len(), 3, "edits were: {edits:?}");
    assert!(edits.iter().all(|e| e.new_text == "hound"));
}

#[test]
fn rename_type_edits_annotations_and_constructors() {
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let edit = rename::rename(
        &doc,
        &krate,
        Some(&main_path()),
        position_of(SOURCE, "Dog", 0),
        "Hound",
    )
    .expect("expected a workspace edit");

    let changes = edit.changes.expect("expected changes");
    let uri = Url::from_file_path(main_path()).unwrap();
    let edits = changes.get(&uri).expect("edits for the main file");
    // Declaration + `self: Dog` twice + constructor `Dog(name: ...)`.
    assert_eq!(edits.len(), 4, "edits were: {edits:?}");
}

#[test]
fn rename_rejects_invalid_identifiers() {
    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    for invalid in ["", "1abc", "a b", "a-b"] {
        let edit = rename::rename(
            &doc,
            &krate,
            Some(&main_path()),
            position_of(SOURCE, "dog", 1),
            invalid,
        );
        assert!(edit.is_none(), "accepted invalid name {invalid:?}");
    }
}

// ----------------------------------------------------------------------
// Symbols
// ----------------------------------------------------------------------

#[test]
fn document_symbols_nest_members_under_their_type() {
    use tower_lsp::lsp_types::SymbolKind;

    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let outline = symbols::document_symbols(&doc, &krate, Some(&main_path()));

    let names: Vec<(&str, SymbolKind)> = outline
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.kind))
        .collect();
    assert!(names.contains(&("greet", SymbolKind::FUNCTION)), "{names:?}");
    assert!(names.contains(&("Dog", SymbolKind::STRUCT)), "{names:?}");

    let dog = outline.iter().find(|symbol| symbol.name == "Dog").unwrap();
    let children: Vec<(&str, SymbolKind)> = dog
        .children
        .as_ref()
        .expect("Dog has members")
        .iter()
        .map(|child| (child.name.as_str(), child.kind))
        .collect();
    assert!(children.contains(&("name", SymbolKind::FIELD)), "{children:?}");
    assert!(children.contains(&("walk", SymbolKind::METHOD)), "{children:?}");
    // Methods nested under their type do not repeat at the top level.
    assert!(!names.iter().any(|(name, _)| *name == "walk"), "{names:?}");
}

#[test]
fn document_symbols_mark_enums() {
    use tower_lsp::lsp_types::SymbolKind;

    let doc = Document::new(ENUM_SOURCE);
    let krate = single_file_crate(ENUM_SOURCE);
    let outline = symbols::document_symbols(&doc, &krate, Some(&main_path()));

    let color = outline.iter().find(|symbol| symbol.name == "Color").unwrap();
    assert_eq!(color.kind, SymbolKind::ENUM);
    let children = color.children.as_ref().expect("Color has cases");
    assert!(
        children
            .iter()
            .any(|child| child.name == "Transparent" && child.kind == SymbolKind::ENUM_MEMBER),
        "children were: {children:?}"
    );
}

#[test]
fn workspace_symbols_filter_by_query() {
    use tower_lsp::lsp_types::SymbolKind;

    let krate = single_file_crate(SOURCE);
    let hits = symbols::workspace_symbols(&krate, "walk");
    assert!(
        hits.iter()
            .any(|hit| hit.name == "walk" && hit.kind == SymbolKind::METHOD),
        "hits were: {hits:?}"
    );
    // Case-insensitive; locals are never workspace symbols.
    let hits = symbols::workspace_symbols(&krate, "dOg");
    assert!(hits.iter().any(|hit| hit.name == "Dog"), "hits: {hits:?}");
    assert!(hits.iter().all(|hit| hit.name != "dog"), "hits: {hits:?}");
}

// ----------------------------------------------------------------------
// Inlay hints
// ----------------------------------------------------------------------

#[test]
fn inlay_hints_show_inferred_types_of_unannotated_lets() {
    use tower_lsp::lsp_types::{InlayHintLabel, Position, Range};

    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let whole_file = Range {
        start: Position::new(0, 0),
        end: Position::new(u32::MAX, 0),
    };
    let hints = inlay_hints::inlay_hints(&doc, &krate, Some(&main_path()), whole_file);

    // `let dog = Dog(...)` has no annotation: a `: Dog` hint after the name.
    let labels: Vec<&str> = hints
        .iter()
        .map(|hint| match &hint.label {
            InlayHintLabel::String(label) => label.as_str(),
            other => panic!("unexpected label: {other:?}"),
        })
        .collect();
    assert!(labels.contains(&": Dog"), "hints were: {labels:?}");
    let dog_hint = hints
        .iter()
        .find(|hint| matches!(&hint.label, InlayHintLabel::String(l) if l == ": Dog"))
        .unwrap();
    let expected = position_of(SOURCE, " = Dog(", 0);
    assert_eq!(dog_hint.position, expected, "hint in the wrong place");
}

#[test]
fn inlay_hints_skip_annotated_bindings() {
    use tower_lsp::lsp_types::{Position, Range};

    let src = "fn f() {\n    let n: Int = 1\n    let m = 2\n}\n";
    let doc = Document::new(src);
    let krate = single_file_crate(src);
    let whole_file = Range {
        start: Position::new(0, 0),
        end: Position::new(u32::MAX, 0),
    };
    let hints = inlay_hints::inlay_hints(&doc, &krate, Some(&main_path()), whole_file);
    // Only the unannotated `m` gets a hint.
    assert_eq!(hints.len(), 1, "hints were: {hints:?}");
}

#[test]
fn inlay_hints_carry_the_annotation_as_text_edit() {
    use tower_lsp::lsp_types::{InlayHintLabel, Position, Range};

    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    let whole_file = Range {
        start: Position::new(0, 0),
        end: Position::new(u32::MAX, 0),
    };
    let hints = inlay_hints::inlay_hints(&doc, &krate, Some(&main_path()), whole_file);
    let dog_hint = hints
        .iter()
        .find(|hint| matches!(&hint.label, InlayHintLabel::String(l) if l == ": Dog"))
        .unwrap();

    let edits = dog_hint.text_edits.as_ref().expect("hint has an edit");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, ": Dog");
    assert_eq!(edits[0].range.start, dog_hint.position);
    assert_eq!(edits[0].range.end, dog_hint.position);
}

// ----------------------------------------------------------------------
// Code actions
// ----------------------------------------------------------------------

#[test]
fn code_action_adds_inferred_type_annotation() {
    use tower_lsp::lsp_types::{CodeActionOrCommand, Range};

    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    // Request on the line of `let dog = Dog(name: "Rex")`.
    let at_binding = position_of(SOURCE, "let dog", 0);
    let actions = code_actions::code_actions(
        &doc,
        &krate,
        Some(&main_path()),
        Range {
            start: at_binding,
            end: at_binding,
        },
        &Default::default(),
    );

    assert_eq!(actions.len(), 1, "actions were: {actions:?}");
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!("expected a code action");
    };
    assert_eq!(action.title, "Add type annotation `: Dog` to `dog`");

    let changes = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .expect("action carries a workspace edit");
    let edits = changes.values().next().unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, ": Dog");
    assert_eq!(edits[0].range.start, position_of(SOURCE, " = Dog(", 0));
}

#[test]
fn code_actions_are_scoped_to_the_requested_lines() {
    use tower_lsp::lsp_types::Range;

    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    // A range on the type declaration: no binding there, no actions.
    let elsewhere = position_of(SOURCE, "type Dog", 0);
    let actions = code_actions::code_actions(
        &doc,
        &krate,
        Some(&main_path()),
        Range {
            start: elsewhere,
            end: elsewhere,
        },
        &Default::default(),
    );
    assert!(actions.is_empty(), "actions were: {actions:?}");
}

#[test]
fn code_action_not_offered_for_annotated_bindings() {
    use tower_lsp::lsp_types::Range;

    let source = "fn f() {\n    let n: Int = 1\n}\n";
    let doc = Document::new(source);
    let krate = single_file_crate(source);
    let at_binding = position_of(source, "let n", 0);
    let actions = code_actions::code_actions(
        &doc,
        &krate,
        Some(&main_path()),
        Range {
            start: at_binding,
            end: at_binding,
        },
        &Default::default(),
    );
    assert!(actions.is_empty(), "actions were: {actions:?}");
}

// ----------------------------------------------------------------------
// Formatting
// ----------------------------------------------------------------------

fn default_format_options() -> tower_lsp::lsp_types::FormattingOptions {
    tower_lsp::lsp_types::FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        ..Default::default()
    }
}

/// Apply `edits` (non-overlapping, as produced by the formatter) to `source`.
fn apply_edits(source: &str, edits: &[tower_lsp::lsp_types::TextEdit]) -> String {
    let doc = Document::new(source);
    let mut byte_edits: Vec<(usize, usize, &str)> = edits
        .iter()
        .map(|edit| {
            (
                doc.line_index.offset(source, edit.range.start).unwrap(),
                doc.line_index.offset(source, edit.range.end).unwrap(),
                edit.new_text.as_str(),
            )
        })
        .collect();
    byte_edits.sort_by_key(|(start, _, _)| *start);

    let mut text = source.to_string();
    for (start, end, new_text) in byte_edits.into_iter().rev() {
        text.replace_range(start..end, new_text);
    }
    text
}

fn format_text(source: &str) -> Option<String> {
    let doc = Document::new(source);
    let edits = formatting::formatting(&doc, &default_format_options())?;
    Some(apply_edits(source, &edits))
}

#[test]
fn formatting_normalizes_indentation() {
    let source = "fn main_fn() {\nlet x = 1\n        let y = 2\n}\n";
    assert_eq!(
        format_text(source).unwrap(),
        "fn main_fn() {\n    let x = 1\n    let y = 2\n}\n"
    );
}

#[test]
fn formatting_collapses_short_member_chains() {
    let source = "fn f() {\n    let x = foo()\n.bar()\n}\n";
    assert_eq!(format_text(source).unwrap(), "fn f() {\n    let x = foo().bar()\n}\n");
}

#[test]
fn formatting_breaks_long_member_chains_one_link_per_line() {
    let source = "fn f(state: ApiState) {\n    let app = router.route(\"/health\", get(health)).route(\"/tickets\", get(list_tickets)).with_state(state)\n}\n";
    assert_eq!(
        format_text(source).unwrap(),
        "fn f(state: ApiState) {
    let app = router.route(\"/health\", get(health))
        .route(\"/tickets\", get(list_tickets))
        .with_state(state)
}
"
    );
}

#[test]
fn formatting_collapses_short_argument_lists() {
    let source = "\
fn f() {
    let state = Api(
        id: 1,
        )
}
";
    assert_eq!(format_text(source).unwrap(), "fn f() {\n    let state = Api(id: 1)\n}\n");
}

#[test]
fn formatting_removes_trailing_whitespace_and_blanks() {
    let source = "fn f() {   \n    let x = 1  \n   \n}\n";
    assert_eq!(format_text(source).unwrap(), "fn f() {\n    let x = 1\n}\n");
}

#[test]
fn formatting_preserves_multiline_string_content() {
    let source = "fn f() {\n    let s = #\"keep\n  weird   \nindent\"#\n}\n";
    // The lines inside the raw string keep their exact whitespace.
    assert_eq!(format_text(source).unwrap(), source);
}

#[test]
fn formatting_refuses_files_that_do_not_parse() {
    let source = "fn broken( {\n";
    let doc = Document::new(source);
    assert!(formatting::formatting(&doc, &default_format_options()).is_none());
}

#[test]
fn formatting_makes_no_edits_on_well_formatted_source_and_is_idempotent() {
    let doc = Document::new(SOURCE);
    let edits = formatting::formatting(&doc, &default_format_options()).unwrap();
    assert!(edits.is_empty(), "unexpected edits: {edits:?}");

    // Idempotence on a source that does need work.
    let messy = "fn main_fn() {\nlet x = foo()\n.bar()   \n}\n";
    let once = format_text(messy).unwrap();
    let twice = format_text(&once).unwrap();
    assert_eq!(once, twice);
}

/// The example projects are hand-formatted; the formatter must agree with
/// them (files the grammar cannot parse yet are skipped — the formatter
/// refuses those by design).
#[test]
fn formatting_leaves_the_example_projects_unchanged() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../example-projects");
    let mut checked = 0;
    for project in std::fs::read_dir(&examples).unwrap() {
        let src = project.unwrap().path().join("src");
        let Ok(files) = std::fs::read_dir(&src) else {
            continue;
        };
        for file in files {
            let path = file.unwrap().path();
            if path.extension().is_none_or(|ext| ext != "galvan") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let doc = Document::new(&source);
            let Some(edits) = formatting::formatting(&doc, &default_format_options()) else {
                continue; // Grammar gap (e.g. `async fn`): formatter refuses.
            };
            assert!(
                edits.is_empty(),
                "formatter wants to change {path:?}: {edits:?}"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no example files were parseable");
}

#[test]
fn formatting_uses_tabs_when_requested() {
    let source = "fn f() {\n    let x = 1\n}\n";
    let doc = Document::new(source);
    let options = tower_lsp::lsp_types::FormattingOptions {
        tab_size: 4,
        insert_spaces: false,
        ..Default::default()
    };
    let edits = formatting::formatting(&doc, &options).unwrap();
    assert_eq!(apply_edits(source, &edits), "fn f() {\n\tlet x = 1\n}\n");
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

// ----------------------------------------------------------------------
// Foreign keywords & quickfixes
// ----------------------------------------------------------------------

fn diagnostics_for(source: &str) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let doc = Document::new(source);
    let krate = single_file_crate(source);
    diagnostics::diagnostics(&doc, &krate, Some(&main_path()))
}

/// Quickfix actions produced by round-tripping `source`'s diagnostics
/// through the code-action context, the way a client would.
fn quickfixes_for(source: &str) -> Vec<tower_lsp::lsp_types::CodeAction> {
    use tower_lsp::lsp_types::{CodeActionContext, CodeActionOrCommand, Range};

    let doc = Document::new(source);
    let krate = single_file_crate(source);
    let context = CodeActionContext {
        diagnostics: diagnostics_for(source),
        ..Default::default()
    };
    let whole_file = Range {
        start: Position::new(0, 0),
        end: Position::new(u32::MAX, 0),
    };
    code_actions::code_actions(&doc, &krate, Some(&main_path()), whole_file, &context)
        .into_iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action)
                if action.kind == Some(tower_lsp::lsp_types::CodeActionKind::QUICKFIX) =>
            {
                Some(action)
            }
            _ => None,
        })
        .collect()
}

fn quickfix_edit(action: &tower_lsp::lsp_types::CodeAction) -> &tower_lsp::lsp_types::TextEdit {
    let changes = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .expect("quickfix carries a workspace edit");
    &changes.values().next().unwrap()[0]
}

#[test]
fn foreign_keyword_in_unparseable_code_is_explained() {
    let source = "func greet() {\n}\n";
    let diags = diagnostics_for(source);

    assert_eq!(diags.len(), 1, "diagnostics: {diags:?}");
    let diag = &diags[0];
    assert_eq!(
        diag.message,
        "`func` is not a Galvan keyword — Galvan uses `fn`"
    );
    assert_eq!(
        diag.code,
        Some(tower_lsp::lsp_types::NumberOrString::String(
            "foreign_keyword".into()
        ))
    );
    // Narrowed to the keyword, not the whole error region.
    assert_eq!(diag.range.start, Position::new(0, 0));
    assert_eq!(diag.range.end, Position::new(0, 4));
}

#[test]
fn foreign_keyword_parsing_as_a_call_is_explained() {
    // `switch color { }` parses as a trailing-closure call, so the parser
    // never complains; detection goes through the unresolved callee.
    let source = "fn main_fn() {\n    let color = 1\n    switch color {\n    }\n}\n";
    let diags = diagnostics_for(source);

    let foreign: Vec<_> = diags
        .iter()
        .filter(|d| d.message.contains("not a Galvan keyword"))
        .collect();
    assert_eq!(foreign.len(), 1, "diagnostics: {diags:?}");
    assert_eq!(
        foreign[0].message,
        "`switch` is not a Galvan keyword — Galvan uses `match`"
    );
    assert_eq!(foreign[0].range.start, position_of(source, "switch", 0));
}

#[test]
fn foreign_keyword_in_expression_position_is_explained() {
    // `null` parses as a plain identifier and fails typechecking; the
    // unknown-identifier diagnostic is rewritten to explain the mapping.
    let source = "fn main_fn() {\n    let x = null\n}\n";
    let diags = diagnostics_for(source);

    assert_eq!(diags.len(), 1, "diagnostics: {diags:?}");
    assert_eq!(
        diags[0].message,
        "`null` is not a Galvan keyword — Galvan uses `none`"
    );
}

#[test]
fn identifiers_matching_foreign_keywords_are_legal_in_valid_code() {
    // `def` is a perfectly legal Galvan identifier; only error regions and
    // unresolved names are checked.
    let source = "fn main_fn() {\n    let def = 1\n    println(def)\n}\n";
    let diags = diagnostics_for(source);
    assert!(diags.is_empty(), "diagnostics: {diags:?}");
}

#[test]
fn foreign_keywords_in_comments_are_not_flagged() {
    // The comment mentions `class` right next to broken code; only the
    // actual foreign keyword is reported.
    let source = "// a class comment\nfunc greet() {\n}\n";
    let diags = diagnostics_for(source);

    assert_eq!(diags.len(), 1, "diagnostics: {diags:?}");
    assert!(diags[0].message.contains("`func`"), "got: {}", diags[0].message);
}

#[test]
fn quickfix_replaces_foreign_keyword() {
    let source = "func greet() {\n}\n";
    let fixes = quickfixes_for(source);

    assert_eq!(fixes.len(), 1, "quickfixes: {fixes:?}");
    let fix = &fixes[0];
    assert_eq!(fix.title, "Replace `func` with `fn`");
    assert_eq!(fix.is_preferred, Some(true));
    assert_eq!(fix.diagnostics.as_ref().map(Vec::len), Some(1));

    let edit = quickfix_edit(fix);
    assert_eq!(edit.new_text, "fn");
    let fixed = apply_edits(source, std::slice::from_ref(edit));
    assert_eq!(fixed, "fn greet() {\n}\n");
    assert!(
        diagnostics_for(&fixed).is_empty(),
        "applying the fix should produce a clean file"
    );
}

#[test]
fn quickfix_applies_did_you_mean_suggestions() {
    let source = "fn main_fn() {\n    let color = 1\n    println(colr)\n}\n";
    let fixes = quickfixes_for(source);

    assert_eq!(fixes.len(), 1, "quickfixes: {fixes:?}");
    let fix = &fixes[0];
    assert_eq!(fix.title, "Replace `colr` with `color`");

    let edit = quickfix_edit(fix);
    let fixed = apply_edits(source, std::slice::from_ref(edit));
    assert!(
        diagnostics_for(&fixed).is_empty(),
        "applying the fix should produce a clean file"
    );
}

// ----------------------------------------------------------------------
// Signature help
// ----------------------------------------------------------------------

/// LSP position just past the `nth` occurrence of `needle`.
fn position_after(text: &str, needle: &str, nth: usize) -> Position {
    let byte = byte_of(text, needle, nth) + needle.len();
    Document::new(text).line_index.position(text, byte)
}

fn help_at(source: &str, position: Position) -> Option<SignatureHelp> {
    let doc = Document::new(source);
    let krate = single_file_crate(source);
    signature_help::signature_help(&doc, &krate, Some(&main_path()), position)
}

fn active_label(help: &SignatureHelp) -> &str {
    &help.signatures[help.active_signature.unwrap_or(0) as usize].label
}

#[test]
fn signature_help_for_free_function_call() {
    // Inside `greet(name)` in the body of `greet`.
    let help = help_at(SOURCE, position_after(SOURCE, "greet(", 1)).expect("expected help");
    assert_eq!(active_label(&help), "fn greet(name: String)");
    assert_eq!(help.active_parameter, Some(0));
}

#[test]
fn signature_help_shows_receiver_but_does_not_count_it() {
    // Inside `dog.walk(5)`: `self: Dog` is shown in the label, but argument 0
    // is `distance`.
    let help = help_at(SOURCE, position_after(SOURCE, "dog.walk(", 0)).expect("expected help");
    let label = active_label(&help);
    assert_eq!(label, "fn walk(self: Dog, distance: Int)");

    let signature = &help.signatures[0];
    let params = signature.parameters.as_ref().unwrap();
    assert_eq!(params.len(), 1, "receiver must not be a parameter");
    let expected_start = label.find("distance: Int").unwrap() as u32;
    let expected_end = expected_start + "distance: Int".len() as u32;
    assert_eq!(
        params[0].label,
        ParameterLabel::LabelOffsets([expected_start, expected_end])
    );
    assert_eq!(help.active_parameter, Some(0));
}

#[test]
fn signature_help_advances_on_commas_and_renders_return_type() {
    let source = "\
fn add(a: Int, b: Int) -> Int {
    a + b
}

fn main_fn() {
    let x = add(1, 2)
}
";
    let help = help_at(source, position_after(source, "add(1, ", 0)).expect("expected help");
    assert_eq!(active_label(&help), "fn add(a: Int, b: Int) -> Int");
    assert_eq!(help.active_parameter, Some(1));
}

#[test]
fn signature_help_filters_methods_by_receiver_type() {
    let source = "\
type Dog {
    name: String
}

type Cat {
    name: String
}

fn walk(self: Dog, distance: Int) {
}

fn walk(self: Cat, distance: Int) {
}

fn main_fn() {
    let dog = Dog(name: \"Rex\")
    dog.walk(5)
}
";
    let help = help_at(source, position_after(source, "dog.walk(", 0)).expect("expected help");
    assert_eq!(help.signatures.len(), 1, "only Dog's walk expected");
    assert_eq!(active_label(&help), "fn walk(self: Dog, distance: Int)");
}

#[test]
fn signature_help_for_struct_constructor() {
    let help = help_at(SOURCE, position_after(SOURCE, "Dog(", 0)).expect("expected help");
    assert_eq!(active_label(&help), "Dog(name: String)");
    assert_eq!(help.active_parameter, Some(0));
}

#[test]
fn signature_help_matches_labelled_constructor_arguments_by_name() {
    let source = "\
type Point {
    x: Int,
    y: Int,
}

fn main_fn() {
    let p = Point(y: 2, x: 1)
}
";
    // The cursor is on the *first* argument positionally, but it is labelled
    // `y:`, which is the *second* field.
    let help = help_at(source, position_after(source, "Point(y: 2", 0)).expect("expected help");
    assert_eq!(help.active_parameter, Some(1));
}

#[test]
fn signature_help_for_enum_case_constructor() {
    let source = "\
type Shade {
    Rgb(r: Int, g: Int, b: Int),
    Gray,
}

fn main_fn() {
    let color = Shade::Rgb(r: 1, g: 2, b: 3)
}
";
    let help =
        help_at(source, position_after(source, "Shade::Rgb(r: 1, ", 0)).expect("expected help");
    assert_eq!(active_label(&help), "Shade::Rgb(r: Int, g: Int, b: Int)");
    assert_eq!(help.active_parameter, Some(1));
}

#[test]
fn signature_help_survives_parse_error_when_siblings_parse() {
    // The current file has a dangling `helper(` and does not parse; the
    // declaration lives in a sibling file.
    let broken = "fn main_fn() {\n    helper(\n}\n";
    let other_path = PathBuf::from("/galvan_lsp_test/src/other.galvan");
    let krate = Crate::in_memory([
        (main_path(), broken.to_string()),
        (
            other_path,
            "fn helper(a: Int, b: Int) {\n}\n".to_string(),
        ),
    ]);
    let doc = Document::new(broken);
    let help = signature_help::signature_help(
        &doc,
        &krate,
        Some(&main_path()),
        position_after(broken, "helper(", 0),
    )
    .expect("expected help from the sibling file");
    assert_eq!(active_label(&help), "fn helper(a: Int, b: Int)");
    assert_eq!(help.active_parameter, Some(0));
}

#[test]
fn signature_help_ignores_commas_in_nested_literals_and_strings() {
    let source = "\
fn plot(points: [Int], label: String) {
}

fn main_fn() {
    plot([1, 2], \"a, b\")
}
";
    // Inside the array literal: still argument 0.
    let help = help_at(source, position_after(source, "plot([1, ", 0)).expect("expected help");
    assert_eq!(help.active_parameter, Some(0), "comma inside `[]`");

    // Inside the string literal (after its comma): argument 1.
    let help = help_at(source, position_after(source, "\"a, ", 0)).expect("expected help");
    assert_eq!(help.active_parameter, Some(1), "comma inside a string");
}

#[test]
fn signature_help_beyond_last_parameter_highlights_nothing() {
    let source = "\
fn greet(name: String) {
}

fn main_fn() {
    greet(\"a\", \"b\")
}
";
    let help = help_at(source, position_after(source, "greet(\"a\", ", 0)).expect("expected help");
    assert_eq!(help.active_parameter, None);
}

#[test]
fn signature_help_selects_matching_overload() {
    let source = "\
fn area(width w: Int, height h: Int) -> Int {
    w * h
}

fn area(radius r: Int) -> Int {
    r * r
}

fn main_fn() {
    let x = area(radius: 2)
}
";
    let help = help_at(source, position_after(source, "area(radius: 2", 0)).expect("expected help");
    assert_eq!(help.signatures.len(), 2, "both overloads offered");
    // The labelled argument `radius:` selects the single-parameter overload.
    assert_eq!(active_label(&help), "fn area(radius r: Int) -> Int");
    assert_eq!(help.active_parameter, Some(0));
}

#[test]
fn no_signature_help_outside_a_call() {
    assert!(help_at(SOURCE, position_of(SOURCE, "type Dog", 0)).is_none());
    // Behind the closed call `greet(name)` there is no open argument list.
    assert!(help_at(SOURCE, position_after(SOURCE, "greet(name)", 0)).is_none());
}

// ----------------------------------------------------------------------
// Semantic tokens
// ----------------------------------------------------------------------

/// A decoded semantic token: the byte range it covers plus its legend indices.
struct DecodedToken {
    range: (usize, usize),
    token_type: u32,
    modifiers: u32,
}

/// Run the semantic-tokens feature and undo the LSP delta encoding.
fn decoded_tokens(source: &str) -> Vec<DecodedToken> {
    let doc = Document::new(source);
    let krate = single_file_crate(source);
    let tokens = semantic_tokens::semantic_tokens(&doc, &krate, Some(&main_path()));

    let mut line = 0u32;
    let mut character = 0u32;
    tokens
        .data
        .iter()
        .map(|token| {
            if token.delta_line > 0 {
                line += token.delta_line;
                character = token.delta_start;
            } else {
                character += token.delta_start;
            }
            let start = doc
                .line_index
                .offset(source, Position { line, character })
                .unwrap();
            let end = doc
                .line_index
                .offset(
                    source,
                    Position {
                        line,
                        character: character + token.length,
                    },
                )
                .unwrap();
            DecodedToken {
                range: (start, end),
                token_type: token.token_type,
                modifiers: token.token_modifiers_bitset,
            }
        })
        .collect()
}

/// The `(token_type, modifiers)` of the token covering the `nth` occurrence
/// of `needle`.
fn token_at(source: &str, needle: &str, nth: usize) -> Option<(u32, u32)> {
    let byte = byte_of(source, needle, nth);
    decoded_tokens(source)
        .iter()
        .find(|token| token.range.0 <= byte && byte < token.range.1)
        .map(|token| (token.token_type, token.modifiers))
}

/// Legend index of `token_type` (tests stay valid if the legend is reordered).
fn legend_index(token_type: &tower_lsp::lsp_types::SemanticTokenType) -> u32 {
    semantic_tokens::legend()
        .token_types
        .iter()
        .position(|t| t == token_type)
        .expect("token type missing from legend") as u32
}

const DECLARATION: u32 = 1 << 0;
const DEFAULT_LIBRARY: u32 = 1 << 1;

#[test]
fn semantic_tokens_classify_declarations_and_references() {
    use tower_lsp::lsp_types::SemanticTokenType as T;

    // Keywords.
    assert_eq!(token_at(SOURCE, "fn", 0), Some((legend_index(&T::KEYWORD), 0)));
    assert_eq!(token_at(SOURCE, "let", 0), Some((legend_index(&T::KEYWORD), 0)));
    // Function: declaration vs call.
    assert_eq!(
        token_at(SOURCE, "greet", 0),
        Some((legend_index(&T::FUNCTION), DECLARATION))
    );
    assert_eq!(token_at(SOURCE, "greet", 1), Some((legend_index(&T::FUNCTION), 0)));
    // Method call.
    assert_eq!(token_at(SOURCE, "dog.walk", 0).map(|_| ()), Some(()));
    assert_eq!(
        token_at(SOURCE, "walk(5", 0),
        Some((legend_index(&T::METHOD), 0))
    );
    // User type: declaration and use.
    assert_eq!(
        token_at(SOURCE, "Dog", 0),
        Some((legend_index(&T::STRUCT), DECLARATION))
    );
    assert_eq!(token_at(SOURCE, "Dog", 1), Some((legend_index(&T::STRUCT), 0)));
    // Builtin type and builtin function.
    assert_eq!(
        token_at(SOURCE, "String", 0),
        Some((legend_index(&T::STRUCT), DEFAULT_LIBRARY))
    );
    assert_eq!(
        token_at(SOURCE, "println", 0),
        Some((legend_index(&T::FUNCTION), DEFAULT_LIBRARY))
    );
    // Parameter, local variable, field access.
    assert_eq!(
        token_at(SOURCE, "name", 1), // `greet(name)`
        Some((legend_index(&T::PARAMETER), 0))
    );
    assert_eq!(
        token_at(SOURCE, "dog", 0), // `let dog`
        Some((legend_index(&T::VARIABLE), DECLARATION))
    );
    assert_eq!(
        token_at(SOURCE, "name)", 1), // `println(dog.name)`
        Some((legend_index(&T::PROPERTY), 0))
    );
    // String and number literals.
    assert_eq!(token_at(SOURCE, "\"Rex\"", 0), Some((legend_index(&T::STRING), 0)));
    assert_eq!(token_at(SOURCE, "5", 0), Some((legend_index(&T::NUMBER), 0)));
}

#[test]
fn semantic_tokens_classify_enums_and_cases() {
    use tower_lsp::lsp_types::SemanticTokenType as T;
    let source = "\
type Shade {
    Gray,
    Rgb(r: Int, g: Int, b: Int),
}

fn main_fn() {
    let s = Shade::Gray
}
";
    assert_eq!(
        token_at(source, "Shade", 0),
        Some((legend_index(&T::ENUM), DECLARATION))
    );
    assert_eq!(token_at(source, "Shade", 1), Some((legend_index(&T::ENUM), 0)));
    assert_eq!(
        token_at(source, "Gray", 1),
        Some((legend_index(&T::ENUM_MEMBER), 0))
    );
}

#[test]
fn semantic_tokens_treat_contextual_control_words_as_keywords() {
    use tower_lsp::lsp_types::SemanticTokenType as T;
    let source = "\
fn main_fn() {
    if true {
        println \"hi\"
    }
}
";
    assert_eq!(token_at(source, "if", 0), Some((legend_index(&T::KEYWORD), 0)));
    assert_eq!(token_at(source, "true", 0), Some((legend_index(&T::KEYWORD), 0)));
}

#[test]
fn semantic_tokens_keep_code_inside_string_interpolation() {
    use tower_lsp::lsp_types::SemanticTokenType as T;
    let source = "\
fn shout(name: String) {
    println \"Hello \\(name)!\"
}
";
    // The literal parts are strings; the interpolated expression is not.
    assert_eq!(token_at(source, "Hello", 0), Some((legend_index(&T::STRING), 0)));
    assert_eq!(token_at(source, "!\"", 0), Some((legend_index(&T::STRING), 0)));
    assert_eq!(
        token_at(source, "name)", 0),
        Some((legend_index(&T::PARAMETER), 0))
    );
}

#[test]
fn semantic_tokens_split_multiline_strings_per_line() {
    use tower_lsp::lsp_types::SemanticTokenType as T;
    let source = "fn main_fn() {\n    let s = #\"line one\nline two\"#\n}\n";
    assert_eq!(
        token_at(source, "line one", 0),
        Some((legend_index(&T::STRING), 0))
    );
    assert_eq!(
        token_at(source, "line two", 0),
        Some((legend_index(&T::STRING), 0))
    );
    // No token may span the newline (clients reject multi-line tokens).
    let newline = byte_of(source, "\nline two", 0);
    for token in decoded_tokens(source) {
        assert!(
            !(token.range.0 <= newline && newline < token.range.1),
            "token {:?} spans a newline",
            token.range
        );
    }
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

// ----------------------------------------------------------------------
// Document highlight
// ----------------------------------------------------------------------

#[test]
fn document_highlight_marks_definition_write_and_uses_read() {
    use tower_lsp::lsp_types::DocumentHighlightKind;
    use galvan_lsp::features::document_highlight;

    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    // On a *use* of `dog`; expect the binding plus both uses, in one file.
    let position = position_of(SOURCE, "dog.walk", 0);
    let highlights =
        document_highlight::document_highlight(&doc, &krate, Some(&main_path()), position);

    assert_eq!(highlights.len(), 3, "highlights: {highlights:?}");
    let writes: Vec<_> = highlights
        .iter()
        .filter(|h| h.kind == Some(DocumentHighlightKind::WRITE))
        .collect();
    assert_eq!(writes.len(), 1, "exactly the binding is a write");
    assert_eq!(writes[0].range.start, position_of(SOURCE, "dog = Dog(", 0));
}

// ----------------------------------------------------------------------
// Folding ranges
// ----------------------------------------------------------------------

#[test]
fn folding_ranges_cover_bodies_and_runs() {
    use tower_lsp::lsp_types::FoldingRangeKind;
    use galvan_lsp::features::folding_range;

    let source = "\
use foo::bar
use baz::qux

// one
// two
// three
fn f() {
    let xs = [
        1,
        2,
    ]
}
";
    let doc = Document::new(source);
    let ranges = folding_range::folding_ranges(&doc);

    // The two use lines fold as imports.
    assert!(
        ranges.iter().any(|r| r.kind == Some(FoldingRangeKind::Imports)
            && (r.start_line, r.end_line) == (0, 1)),
        "ranges: {ranges:?}"
    );
    // The comment block folds as a comment.
    assert!(
        ranges.iter().any(|r| r.kind == Some(FoldingRangeKind::Comment)
            && (r.start_line, r.end_line) == (3, 5)),
        "ranges: {ranges:?}"
    );
    // The function body folds from its `{` to the line before the `}`.
    assert!(
        ranges.iter().any(|r| r.kind == Some(FoldingRangeKind::Region)
            && (r.start_line, r.end_line) == (6, 10)),
        "ranges: {ranges:?}"
    );
    // The multi-line collection literal folds too.
    assert!(
        ranges.iter().any(|r| r.kind == Some(FoldingRangeKind::Region)
            && (r.start_line, r.end_line) == (7, 9)),
        "ranges: {ranges:?}"
    );
}

// ----------------------------------------------------------------------
// Selection ranges
// ----------------------------------------------------------------------

#[test]
fn selection_range_expands_outward_to_the_whole_file() {
    use galvan_lsp::features::selection_range;

    let doc = Document::new(SOURCE);
    // Inside `name` of `greet(self.name)`.
    let position = position_of(SOURCE, "name)", 1);
    let innermost =
        selection_range::selection_range(&doc, position).expect("selection range resolved");

    // Walk outward: every step must contain the previous one, and the
    // outermost range is the whole file.
    let mut ranges = vec![innermost.range];
    let mut current = innermost.parent;
    while let Some(next) = current {
        ranges.push(next.range);
        current = next.parent;
    }
    assert!(ranges.len() >= 4, "expected several steps, got: {ranges:?}");
    for pair in ranges.windows(2) {
        assert!(
            pair[1].start <= pair[0].start && pair[0].end <= pair[1].end,
            "each step contains the previous: {ranges:?}"
        );
    }
    let outermost = ranges.last().unwrap();
    assert_eq!(outermost.start, Position::new(0, 0));
    // The innermost selects the identifier under the cursor.
    assert_eq!(ranges[0].start, position_of(SOURCE, "name)", 1));
}

// ----------------------------------------------------------------------
// Type definition
// ----------------------------------------------------------------------

#[test]
fn type_definition_jumps_from_variable_to_its_type() {
    use galvan_lsp::features::type_definition;

    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    // On a use of `dog` (a `Dog`): jump to `type Dog`.
    let position = position_of(SOURCE, "dog.walk", 0);
    let location = type_definition::type_definition(&doc, &krate, Some(&main_path()), position)
        .expect("type definition resolved");
    assert_eq!(location.range.start, position_of(SOURCE, "Dog {", 0));
}

#[test]
fn type_definition_jumps_from_parameter_to_its_type() {
    use galvan_lsp::features::type_definition;

    let doc = Document::new(SOURCE);
    let krate = single_file_crate(SOURCE);
    // On the use of `self` inside `pet` (`self: Dog`).
    let position = position_of(SOURCE, "self.name", 0);
    let location = type_definition::type_definition(&doc, &krate, Some(&main_path()), position)
        .expect("type definition resolved");
    assert_eq!(location.range.start, position_of(SOURCE, "Dog {", 0));
}

// ----------------------------------------------------------------------
// Range & on-type formatting
// ----------------------------------------------------------------------

#[test]
fn range_formatting_touches_only_the_requested_lines() {
    use tower_lsp::lsp_types::Range;

    // Two misindented lines; only the second is in the requested range.
    let source = "fn f() {\n        let a = 1\n        let b = 2\n}\n";
    let doc = Document::new(source);
    let range = Range {
        start: Position::new(2, 0),
        end: Position::new(2, 0),
    };
    let edits = formatting::range_formatting(&doc, &default_format_options(), range)
        .expect("file parses");

    assert_eq!(edits.len(), 1, "edits: {edits:?}");
    assert_eq!(edits[0].range.start.line, 2);
    // A full format agrees but also fixes line 1.
    let full = formatting::formatting(&doc, &default_format_options()).unwrap();
    assert_eq!(full.len(), 2);
    assert!(full.contains(&edits[0]));
}

#[test]
fn on_type_formatting_dedents_a_closing_brace() {
    // The `}` was just typed with too much indentation.
    let source = "fn f() {\n    let a = 1\n        }\n";
    let doc = Document::new(source);
    let edits =
        formatting::on_type_formatting(&doc, &default_format_options(), Position::new(2, 9))
            .expect("file parses");

    assert_eq!(edits.len(), 1, "edits: {edits:?}");
    assert_eq!(edits[0].range.start, Position::new(2, 0));
    assert_eq!(edits[0].range.end, Position::new(2, 8));
    assert_eq!(edits[0].new_text, "");
    // Other lines are untouched even though the whole file was analysed.
    assert!(edits.iter().all(|e| e.range.start.line == 2));
}

mod rust_interop_features {
    //! Interop-backed features: completion, hover and go-to-definition for
    //! symbols lifted from Rust crates, exercised with an injected interop
    //! (no cargo/rustdoc involved).

    use super::*;
    use galvan_ast::{
        BasicTypeItem, EmptyTypeDecl, FnDecl, FnSignature, Ident, ParamList, Span, ToplevelItem,
        TypeDecl, TypeElement, TypeIdent, UseDecl, UsePath, Visibility,
    };
    use galvan_files::Source;
    use galvan_rustdoc::{RustInterop, RustSourceSpan};

    const INTEROP_SOURCE: &str = "\
use external::Dog

fn main_fn() {
    let dog = Dog.new()
    let s = external::fetch()
}
";

    fn plain(name: &str) -> TypeElement {
        TypeElement::Plain(BasicTypeItem {
            ident: TypeIdent::new(name),
            span: Span::default(),
        })
    }

    fn signature(name: &str, return_type: TypeElement) -> FnSignature {
        FnSignature {
            visibility: Visibility::public(),
            is_async: false,
            identifier: Ident::new(name),
            parameters: ParamList {
                params: vec![],
                span: Span::default(),
            },
            return_type,
            where_clause: None,
            span: Span::default(),
        }
    }

    fn use_decl(segments: &[&str]) -> ToplevelItem<UseDecl> {
        ToplevelItem {
            item: UseDecl {
                path: UsePath {
                    segments: segments.iter().map(|segment| Ident::new(*segment)).collect(),
                    span: Span::default(),
                },
                span: Span::default(),
            },
            source: Source::Builtin,
        }
    }

    fn interop(new_span: Option<RustSourceSpan>) -> RustInterop {
        let mut interop = RustInterop::empty();
        interop.add_type_decl(
            "external",
            "Dog",
            "::external::Dog",
            TypeDecl::Empty(EmptyTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new("Dog"),
                generic_params: Vec::new(),
                span: Span::default(),
            }),
        );
        interop.add_function_decl(
            "external",
            "fetch",
            "::external::fetch",
            FnDecl::from(signature("fetch", plain("String"))),
            false,
        );
        interop.add_associated_function_decl(
            "external",
            TypeIdent::new("Dog"),
            "new",
            "::external::Dog::new",
            FnDecl::from(signature("new", plain("Dog"))),
            false,
        );
        if let Some(span) = new_span {
            interop
                .functions
                .last_mut()
                .expect("associated fn was just added")
                .source_span = Some(span);
        }
        interop.import_uses(&[use_decl(&["external", "Dog"])]);
        interop
    }

    fn interop_crate(source: &str, new_span: Option<RustSourceSpan>) -> Crate {
        Crate::in_memory_with_interop([(main_path(), source.into())], interop(new_span))
    }

    #[test]
    fn hover_shows_rust_signature_for_associated_call() {
        let doc = Document::new(INTEROP_SOURCE);
        let krate = interop_crate(INTEROP_SOURCE, None);
        let position = position_of(INTEROP_SOURCE, "new", 0);
        let hover = hover::hover(&doc, &krate, Some(&main_path()), position)
            .expect("hover on Dog.new should resolve");
        let HoverContents::Markup(MarkupContent { value, .. }) = hover.contents else {
            panic!("expected markdown hover");
        };
        assert!(
            value.contains("fn new() -> Dog") && value.contains("external"),
            "hover should show the lifted Rust signature, got: {value}"
        );
    }

    #[test]
    fn hover_shows_rust_origin_for_imported_type() {
        let doc = Document::new(INTEROP_SOURCE);
        let krate = interop_crate(INTEROP_SOURCE, None);
        // The `Dog` in `let dog = Dog.new()` (occurrence 1; occurrence 0 is the use).
        let position = position_of(INTEROP_SOURCE, "Dog", 1);
        let hover = hover::hover(&doc, &krate, Some(&main_path()), position)
            .expect("hover on the imported type should resolve");
        let HoverContents::Markup(MarkupContent { value, .. }) = hover.contents else {
            panic!("expected markdown hover");
        };
        assert!(
            value.contains("::external::Dog"),
            "hover should name the Rust type, got: {value}"
        );
    }

    #[test]
    fn goto_definition_jumps_into_rust_sources() {
        // A real file, so the location survives the existence check.
        let rust_file = std::env::temp_dir().join("galvan_lsp_interop_dog.rs");
        std::fs::write(&rust_file, "pub struct Dog;\nimpl Dog { pub fn new() {} }\n").unwrap();

        let doc = Document::new(INTEROP_SOURCE);
        let krate = interop_crate(
            INTEROP_SOURCE,
            Some(RustSourceSpan {
                path: rust_file.clone(),
                line: 2,
                column: 11,
            }),
        );
        let position = position_of(INTEROP_SOURCE, "new", 0);
        let location = goto_definition::goto_definition(&doc, &krate, Some(&main_path()), position)
            .expect("goto definition on Dog.new should jump into the Rust source");
        assert_eq!(location.uri, Url::from_file_path(&rust_file).unwrap());
        // rustdoc lines are 1-based, LSP lines 0-based.
        assert_eq!(location.range.start.line, 1);
        assert_eq!(location.range.start.character, 11);

        let _ = std::fs::remove_file(&rust_file);
    }

    #[test]
    fn path_completion_offers_crate_items() {
        let source = "\
use external::Dog

fn main_fn() {
    let s = external::fetch()
}
";
        let doc = Document::new(source);
        let krate = interop_crate(source, None);
        // Complete right after `external::`.
        let offset = byte_of(source, "external::fetch", 0) + "external::".len();
        let position = Document::new(source).line_index.position(source, offset);
        let items = completion::completion(&doc, &krate, Some(&main_path()), position);
        let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        assert!(
            labels.contains(&"fetch") && labels.contains(&"Dog"),
            "external:: should complete the crate's items, got: {labels:?}"
        );
    }

    #[test]
    fn member_completion_offers_associated_functions() {
        let source = "\
use external::Dog

fn main_fn() {
    let dog = Dog.new()
}
";
        let doc = Document::new(source);
        let krate = interop_crate(source, None);
        let offset = byte_of(source, "Dog.new", 0) + "Dog.".len();
        let position = Document::new(source).line_index.position(source, offset);
        let items = completion::completion(&doc, &krate, Some(&main_path()), position);
        let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        assert!(
            labels.contains(&"new"),
            "Dog. should complete associated functions, got: {labels:?}"
        );
    }

    #[test]
    fn type_completion_offers_imported_rust_types() {
        let source = "\
use external::Dog

fn walk(dog: Dog) {
}
";
        let doc = Document::new(source);
        let krate = interop_crate(source, None);
        // Complete in type position, after the `:` of a parameter.
        let offset = byte_of(source, ": Dog", 0) + 2;
        let position = Document::new(source).line_index.position(source, offset);
        let items = completion::completion(&doc, &krate, Some(&main_path()), position);
        let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        assert!(
            labels.contains(&"Dog"),
            "type completion should offer imported Rust types, got: {labels:?}"
        );
    }
}
