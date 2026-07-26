//! End-to-end tests for the Galvan formatter: construct-by-construct
//! expectations, option handling, and the two invariants every case must
//! hold — idempotency and refusing to touch broken sources.

use galvan_format::{format_source, FormatError, FormatOptions};

fn fmt(src: &str) -> String {
    let formatted = format_source(src, &FormatOptions::default()).expect("input should format");
    let again = format_source(&formatted, &FormatOptions::default())
        .expect("formatted output should still parse and format");
    assert_eq!(formatted, again, "formatting must be idempotent");
    formatted
}

#[test]
fn normalizes_token_spacing() {
    assert_eq!(
        fmt("fn   distance( from:Point ,to : Point )->F64 {\n let dx=from.x-to.x\n(dx * dx)\n}"),
        "fn distance(from: Point, to: Point) -> F64 {\n    let dx = from.x - to.x\n    (dx * dx)\n}\n"
    );
}

#[test]
fn where_clause_and_pub() {
    assert_eq!(
        fmt("pub fn f( x:t )->t where t : Clone + Copy {\nx\n}"),
        "pub fn f(x: t) -> t where t: Clone + Copy {\n    x\n}\n"
    );
}

#[test]
fn struct_fields_one_per_line_with_trailing_comma() {
    assert_eq!(
        fmt("type Point{x:F64,y:F64}"),
        "type Point {\n    x: F64,\n    y: F64,\n}\n"
    );
}

#[test]
fn enum_variants_one_per_line() {
    assert_eq!(
        fmt("type Color {\nRed, Rgb(r:U8,g:U8,b:U8), Gray ( U8 )\n}"),
        "type Color {\n    Red,\n    Rgb(r: U8, g: U8, b: U8),\n    Gray(U8),\n}\n"
    );
}

#[test]
fn alias_and_result_types() {
    assert_eq!(fmt("type Alias=Result!Error"), "type Alias = Result!Error\n");
    assert_eq!(fmt("type A = [ Int ] ?"), "type A = [Int]?\n");
    assert_eq!(fmt("type D = { String : Int }"), "type D = {String: Int}\n");
    assert_eq!(fmt("type V = Vec < Int , String >"), "type V = Vec<Int, String>\n");
}

#[test]
fn statements_split_and_semicolons_dropped() {
    assert_eq!(
        fmt("fn f() { let a = 1 ; let b = 2;println \"x\" }"),
        "fn f() {\n    let a = 1\n    let b = 2\n    println \"x\"\n}\n"
    );
}

#[test]
fn ranges_stay_tight() {
    assert_eq!(
        fmt("fn f() {\nlet a = 0 ..= 5\nlet b=0..<10\n}"),
        "fn f() {\n    let a = 0..=5\n    let b = 0..<10\n}\n"
    );
}

#[test]
fn collection_literals() {
    assert_eq!(
        fmt("fn f() {\nlet c = [ 1,2 , 3 ]\nlet d = { \"a\" : 1 }\nlet e = {:}\nlet o = [ : ]\nlet s = { 1,2 }\n}"),
        "fn f() {\n    let c = [1, 2, 3]\n    let d = {\"a\": 1}\n    let e = {:}\n    let o = [:]\n    let s = {1, 2}\n}\n"
    );
}

#[test]
fn match_arms_inline_when_single_statement() {
    assert_eq!(
        fmt("fn f(x: Int?) -> String {\nmatch  x { _ {   fallback( )   } }\n}"),
        "fn f(x: Int?) -> String {\n    match x {\n        _ { fallback() }\n    }\n}\n"
    );
}

#[test]
fn match_arm_with_multiple_statements_breaks() {
    assert_eq!(
        fmt("fn f(x: Int?) {\nmatch x { _ { foo()\nbar() } }\n}"),
        "fn f(x: Int?) {\n    match x {\n        _ {\n            foo()\n            bar()\n        }\n    }\n}\n"
    );
}

#[test]
fn long_argument_lists_break_with_trailing_comma() {
    assert_eq!(
        fmt("fn f() {\nlet response = TicketResponse(id: id, title: title, priority: priority, status: status, summary: ticket_summary(id, title, priority, status))\n}"),
        "fn f() {
    let response = TicketResponse(
        id: id,
        title: title,
        priority: priority,
        status: status,
        summary: ticket_summary(id, title, priority, status),
    )
}
"
    );
}

#[test]
fn short_argument_lists_collapse() {
    assert_eq!(
        fmt("fn f() {\nlet p = Point(\n    x: 1,\n    y: 2,\n)\n}"),
        "fn f() {\n    let p = Point(x: 1, y: 2)\n}\n"
    );
}

#[test]
fn member_chains_break_keeping_first_link_on_base() {
    assert_eq!(
        fmt("fn f(state: ApiState) {\nlet app = router.route(\"/health\", get(health)).route(\"/tickets\", get(list_tickets).post(create_ticket)).with_state(state)\n}"),
        "fn f(state: ApiState) {
    let app = router.route(\"/health\", get(health))
        .route(\"/tickets\", get(list_tickets).post(create_ticket))
        .with_state(state)
}
"
    );
}

#[test]
fn short_chains_stay_flat() {
    assert_eq!(
        fmt("fn f() {\nlet tiny = a . b .c\nstate.next_id += 1\n}"),
        "fn f() {\n    let tiny = a.b.c\n    state.next_id += 1\n}\n"
    );
}

#[test]
fn trailing_closures_and_else() {
    assert_eq!(
        fmt("fn f(name: String?) {\ntry name |name| {\nprintln \"Hello \\(name)!\"\n} else {\nprintln \"Hello World!\"\n}\n}"),
        "fn f(name: String?) {
    try name |name| {
        println \"Hello \\(name)!\"
    } else {
        println \"Hello World!\"
    }
}
"
    );
}

#[test]
fn else_with_error_binding_collapses_in_expression_position() {
    assert_eq!(
        fmt("fn f() {\nlet payload = serde_json::to_string(scores) else |error| {\n\"encoding failed\"\n}\n}"),
        "fn f() {\n    let payload = serde_json::to_string(scores) else |error| { \"encoding failed\" }\n}\n"
    );
}

#[test]
fn expression_position_control_flow_collapses_when_it_fits() {
    assert_eq!(
        fmt("fn f(dark: Bool) {\nlet color = if dark {\nblue\n} else {\nred\n}\ncolor\n}"),
        "fn f(dark: Bool) {\n    let color = if dark { blue } else { red }\n    color\n}\n"
    );
}

#[test]
fn statement_position_control_flow_always_breaks() {
    assert_eq!(
        fmt("fn f(dark: Bool) {\nif dark { blue }\n}"),
        "fn f(dark: Bool) {\n    if dark {\n        blue\n    }\n}\n"
    );
}

#[test]
fn overlong_expression_position_control_flow_breaks_all_bodies() {
    assert_eq!(
        fmt("fn f() {\nlet value = if some_long_condition_name { compute_something_long(a, b) } else { fallback_value_generator(c) }\n}"),
        "fn f() {
    let value = if some_long_condition_name {
        compute_something_long(a, b)
    } else {
        fallback_value_generator(c)
    }
}
"
    );
}

#[test]
fn blank_lines_preserved_but_capped_at_one() {
    assert_eq!(
        fmt("fn f() {\nlet a = 1\n\n\n\nlet b = 2\n}\n\n\nfn g() {\n}"),
        "fn f() {\n    let a = 1\n\n    let b = 2\n}\n\nfn g() {}\n"
    );
}

#[test]
fn comments_stay_put() {
    assert_eq!(
        fmt("// leading\nfn f() {\nlet b = 2 // trailing\n// own line\nlet c = 3\n}"),
        "// leading\nfn f() {\n    let b = 2 // trailing\n    // own line\n    let c = 3\n}\n"
    );
}

#[test]
fn comments_inside_param_lists_force_break() {
    assert_eq!(
        fmt("cmd main(\n/// Optional name\nn name: String?\n) {\ngreet(name)\n}"),
        "cmd main(\n    /// Optional name\n    n name: String?,\n) {\n    greet(name)\n}\n"
    );
}

#[test]
fn empty_body_collapses() {
    assert_eq!(fmt("fn f() {\n}"), "fn f() {}\n");
    assert_eq!(fmt("fn f() {   }"), "fn f() {}\n");
}

#[test]
fn use_declarations_and_unicode_operators_verbatim() {
    assert_eq!(fmt("use  std :: fmt"), "use std::fmt\n");
    assert_eq!(
        fmt("fn f() -> Bool {\na ≠ b\n}"),
        "fn f() -> Bool {\n    a ≠ b\n}\n"
    );
}

#[test]
fn string_content_untouched() {
    assert_eq!(
        fmt("fn f() {\nprintln \"spaced   out \\(a+b) [1 , 2]\"\n}"),
        "fn f() {\n    println \"spaced   out \\(a+b) [1 , 2]\"\n}\n"
    );
}

#[test]
fn final_newline_is_ensured_and_empty_stays_empty() {
    assert_eq!(fmt("fn f() {\nx\n}"), "fn f() {\n    x\n}\n");
    assert_eq!(fmt(""), "");
}

#[test]
fn syntax_errors_refuse_to_format() {
    let result = format_source("fn f( {", &FormatOptions::default());
    assert!(matches!(result, Err(FormatError::SyntaxErrors)));
}

#[test]
fn operator_spelling_is_untouched_by_default() {
    assert_eq!(
        fmt("fn f(a: Int, b: Int) -> Bool {\na != b and a ≠ b or c ∊ d\n}"),
        "fn f(a: Int, b: Int) -> Bool {\n    a != b and a ≠ b or c ∊ d\n}\n"
    );
}

#[test]
fn operators_normalize_to_unicode_and_words() {
    let options = FormatOptions {
        unicode_operators: galvan_format::UnicodeStyle::Unicode,
        logical_operators: galvan_format::LogicalStyle::Word,
        ..FormatOptions::default()
    };
    let formatted = format_source(
        "fn f(a: Int, b: Int) -> Bool {\nlet x = a != b && c >= d || e ∊ f\nx\n}",
        &options,
    )
    .unwrap();
    assert_eq!(
        formatted,
        "fn f(a: Int, b: Int) → Bool {\n    let x = a ≠ b and c ≥ d or e in f\n    x\n}\n"
    );
}

#[test]
fn operators_normalize_to_ascii_and_symbols() {
    let options = FormatOptions {
        unicode_operators: galvan_format::UnicodeStyle::Ascii,
        logical_operators: galvan_format::LogicalStyle::Symbol,
        ..FormatOptions::default()
    };
    let formatted = format_source(
        "fn f(a: Int, b: Int) → Bool {\nlet x = a ≠ b and c ≥ d or e in f\nx\n}",
        &options,
    )
    .unwrap();
    assert_eq!(
        formatted,
        "fn f(a: Int, b: Int) -> Bool {\n    let x = a != b && c >= d || e ∈ f\n    x\n}\n"
    );
}

#[test]
fn respects_indent_and_width_options() {
    let options = FormatOptions {
        indent_width: 2,
        use_tabs: false,
        max_width: 20,
        ..FormatOptions::default()
    };
    let formatted = format_source("fn f() {\nlet p = Point(x: 1, y: 22)\n}", &options).unwrap();
    assert_eq!(
        formatted,
        "fn f() {\n  let p = Point(\n    x: 1,\n    y: 22,\n  )\n}\n"
    );

    let tabs = FormatOptions {
        use_tabs: true,
        ..FormatOptions::default()
    };
    assert_eq!(
        format_source("fn f() {\nx\n}", &tabs).unwrap(),
        "fn f() {\n\tx\n}\n"
    );
}

#[test]
fn example_projects_format_idempotently() {
    for project in ["hello-world", "cli-app", "serde-json"] {
        let path = format!(
            "{}/../example-projects/{project}/src/main.galvan",
            env!("CARGO_MANIFEST_DIR")
        );
        let source = std::fs::read_to_string(&path).expect("example project source exists");
        let formatted = fmt(&source);
        // The examples are hand-formatted in the intended style already;
        // the only change the formatter should make is adding trailing
        // commas to broken lists.
        for line in formatted.lines() {
            assert!(
                !line.ends_with(' ') && !line.ends_with('\t'),
                "no trailing whitespace in {project}"
            );
        }
    }
}
