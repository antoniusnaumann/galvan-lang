use galvan_test_macro::generate_code_tests;

mod test_utils {
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
}

#[allow(unused_imports)]
use galvan_files::Source;
#[allow(unused_imports)]
use galvan_transpiler::transpile;
use test_utils::*;

generate_code_tests!(test_transpilation, TRANSPILE, trim_all {
    let source = Source::from_string(code);
    let transpilation = transpile(source);
    transpilation.transpiled.unwrap()
});
