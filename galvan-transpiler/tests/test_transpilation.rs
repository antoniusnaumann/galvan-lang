use galvan_test_macro::generate_code_tests;

mod test_utils {
    use galvan_transpiler::TranspileOutput;
    use itertools::Itertools;
    use regex::Regex;

    pub trait MinimalWhitespace {
        fn trim_all(&self) -> String;
    }

    impl MinimalWhitespace for &str {
        fn trim_all(&self) -> String {
            let whitespace = Regex::new(r"\s+").unwrap();
            let newlines = Regex::new(r"\n+").unwrap();
            let trimmed = whitespace.replace_all(self, " ");
            let trimmed = newlines.replace_all(&trimmed, "\n");
            trimmed.trim().to_string()
        }
    }

    impl MinimalWhitespace for String {
        fn trim_all(&self) -> String {
            self.as_str().trim_all()
        }
    }

    pub fn merge_outputs(outputs: Vec<TranspileOutput>) -> String {
        outputs
            .into_iter()
            .map(|output| output.content)
            .collect::<Vec<_>>()
            .join("\n\n")
            .lines()
            .filter(|line| {
                let line = line.trim();
                !line.starts_with("pub use")
                    && !line.starts_with("use")
                    && !line.starts_with("mod")
                    && !line.starts_with("pub(crate) mod")
                    && !line.starts_with("pub mod")
                    && !line.starts_with("#![")
                    && !line.starts_with("extern crate galvan")
                    && !line.starts_with("pub(crate) const __HAS_CLI_COMMANDS")
            })
            .dropping_back(1)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[allow(unused_imports)]
use galvan_files::Source;
#[allow(unused_imports)]
use galvan_transpiler::{galvan_module, transpile};
use test_utils::*;

generate_code_tests!(test_transpilation, TRANSPILE, trim_all {
    let source = Source::from_string(code);
    let transpilation = transpile(vec![source]).unwrap();
    merge_outputs(transpilation)
});

fn transpile_source(code: &str) -> String {
    transpile(vec![Source::from_string(code)])
        .unwrap()
        .into_iter()
        .map(|output| output.content.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn transpiles_main_as_a_normal_function() {
    let output = transpile_source("fn main() { print \"Hello\" }");

    assert!(output.contains("pub(crate) fn __main__()"));
    assert!(!output.contains("std::env::args()"));
}

#[test]
fn collects_argv_for_main_function_argument() {
    let output = transpile_source("fn main(args: [String]) { print args }");

    assert!(output.contains("let args: ::std::vec::Vec<String> = ::std::env::args().collect()"));
}

#[test]
fn transpiles_command_main_arguments_as_top_level_flags() {
    let output = transpile_source(
        "cmd main(
            n name: String,
            count: Int?
        ) {
            print name
        }",
    );

    assert!(output.contains("fn __main_command(name: String, count: Option<i64>)"));
    assert!(output.contains("pub name: String"));
    assert!(output.contains("pub count: Option<i64>"));
    assert!(output.contains("let Cli { name, count } = cli"));
    assert!(output.contains("__main_command(name, count)"));
    assert!(!output.contains("enum Commands"));
}

#[test]
fn command_main_coexists_with_subcommands() {
    let output = transpile_source(
        "cmd main(verbose: Bool?) {}
         cmd greet(name: String) { print name }",
    );

    assert!(output.contains("subcommand_negates_reqs = true"));
    assert!(output.contains("enum Commands"));
    assert!(output.contains("None => __main_command(verbose)"));
}

#[test]
fn transpiles_labeled_function_overloads() {
    let output = transpile_source(
        "fn foo(bar: U8) -> U8 { bar }
         fn foo(bar: U8, num baz: U8) -> U8 { baz }
         fn foo(bar: U8, num baz: U8, ~ msg: U8) -> U8 { msg }
         fn calls() -> U8 {
             let one = foo(5)
             let two = foo(6, num: 6)
             foo(7, num: 8, msg: 9)
         }",
    );

    assert!(output.contains("fn foo(bar: u8) -> u8"));
    assert!(output.contains("fn foo__num(bar: u8, baz: u8) -> u8"));
    assert!(output.contains("fn foo__num__msg(bar: u8, baz: u8, msg: u8) -> u8"));
    assert!(output.contains("foo(5)"));
    assert!(output.contains("foo__num(6, 6)"));
    assert!(output.contains("foo__num__msg(7, 8, 9)"));
}

#[test]
fn transpiles_labeled_method_overloads() {
    let output = transpile_source(
        "type Dog { age: U8 }
         fn age(self: Dog) -> U8 { self.age }
         fn age(self: Dog, by years: U8) -> U8 { self.age + years }
         fn calls(dog: Dog) -> U8 {
             dog.age(by: 2)
         }",
    );

    assert!(output.contains("fn age(&self) -> u8"));
    assert!(output.contains("fn age__by(&self, years: u8) -> u8"));
    assert!(output.contains(".age__by(2)"));
}

#[test]
fn transpiles_import_declarations() {
    let output = transpile_source(
        "use reader
         use reader::score
         fn call() {
             score()
         }",
    );

    assert!(output.contains("use reader::*;"));
    assert!(output.contains("use reader::score;"));
    assert!(output.contains("score()"));
}

#[test]
fn transpiles_qualified_function_calls_without_imports() {
    let output = transpile_source(
        "fn call() {
             reader::score()
         }",
    );

    assert!(output.contains("reader::score()"));
}

#[test]
fn transpiles_namespaced_method_calls_as_scoped_imports() {
    let output = transpile_source(
        "type Book
         fn call(book: Book) {
             book.reader::read_and_judge()
             book.reader::score(with: 5)
         }",
    );

    assert!(output.contains("{ use reader::*; book.read_and_judge() }"));
    assert!(output.contains("{ use reader::*; book.score__with(5) }"));
}

#[test]
fn primitive_ref_struct_fields_use_atomic_storage() {
    let output = transpile_source(
        "type State {
             ref next_id: U64,
             ref active: Bool,
         }
         fn state() -> State {
             State(next_id: 1, active: true)
         }",
    );

    assert!(output.contains("pub(crate) next_id: std::sync::Arc<std::sync::atomic::AtomicU64>"));
    assert!(output.contains("pub(crate) active: std::sync::Arc<std::sync::atomic::AtomicBool>"));
    assert!(output.contains("next_id: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1))"));
    assert!(
        output.contains("active: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true))")
    );
}

#[test]
fn non_primitive_ref_struct_fields_keep_mutex_storage() {
    let output = transpile_source(
        "type Dog {
             name: String,
         }
         type Shelter {
             ref dog: Dog,
         }
         fn shelter() -> Shelter {
             Shelter(dog: Dog(name: \"Rex\"))
         }",
    );

    assert!(output.contains("pub(crate) dog: std::sync::Arc<std::sync::Mutex<Dog>>"));
    assert!(output.contains("dog:"));
    assert!(output.contains(".__to_ref()"));
    assert!(!output.contains("Atomic"));
}

#[test]
fn primitive_ref_locals_and_params_use_atomic_storage() {
    let output = transpile_source(
        "fn increment_ref(ref counter: Int) {
             counter += 1
         }
         fn check() {
             ref counter = 0
             counter = 42
             counter += 1
             assert counter == 43
         }",
    );

    assert!(
        output.contains("fn increment_ref(counter: std::sync::Arc<std::sync::atomic::AtomicI64>)")
    );
    assert!(output.contains(
        "let mut counter: std::sync::Arc<std::sync::atomic::AtomicI64> = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(0))"
    ));
    assert!(output.contains("counter.store(42, std::sync::atomic::Ordering::Relaxed)"));
    assert!(output.contains("counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)"));
    assert!(output.contains("counter.load(std::sync::atomic::Ordering::Relaxed)"));
}

#[test]
fn primitive_ref_mut_arguments_store_back_through_atomic_storage() {
    let output = transpile_source(
        "fn bump(mut value: Int) {
             value += 1
         }
         fn check() {
             ref counter = 0
             bump(counter.mut)
         }",
    );

    assert!(output.contains(
        "let mut __galvan_atomic_arg_0 = counter.load(std::sync::atomic::Ordering::Relaxed)"
    ));
    assert!(output.contains("bump(&mut __galvan_atomic_arg_0)"));
    assert!(output
        .contains("counter.store(__galvan_atomic_arg_0, std::sync::atomic::Ordering::Relaxed)"));
}

#[test]
fn clones_implicitly_for_move_parameters() {
    let output = transpile_source(
        "fn keep(move message: String) -> String {
             message
         }
         fn call() -> String {
             let message = \"hello\"
             keep(message)
         }",
    );

    assert!(output.contains("fn keep(message: String) -> String"));
    assert!(output.contains("keep(message.to_owned())"));
}

#[test]
fn passes_move_arguments_without_implicit_clone() {
    let output = transpile_source(
        "fn keep(move message: String) -> String {
             message
         }
         fn call() -> String {
             let message = \"hello\"
             keep(move message)
         }",
    );

    assert!(output.contains("fn keep(message: String) -> String"));
    assert!(output.contains("keep(message)"));
    assert!(!output.contains("keep(message.to_owned())"));
}

#[test]
fn rejects_double_underscore_identifiers() {
    assert!(transpile(vec![Source::from_string("fn bad__name() {}")]).is_err());
    assert!(transpile(vec![Source::from_string("type Bad__Name {}")]).is_err());
}

#[test]
fn rejects_invalid_main_function_signatures() {
    assert!(transpile(vec![Source::from_string("fn main(value: Int) {}")]).is_err());
    assert!(transpile(vec![Source::from_string("main {}")]).is_err());
    assert!(transpile(vec![Source::from_string("fn main() {} cmd main() {}")]).is_err());
}
