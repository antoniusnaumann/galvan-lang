use galvan_test_macro::generate_code_tests;

use galvan_ast::*;
use galvan_pest::*;

generate_code_tests! { test_ast_conversion, AST,
    parse_source(&Source::from_string(code)).unwrap().try_into_ast().unwrap()
}