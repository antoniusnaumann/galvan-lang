use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use core::panic;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Result, Token};

struct MacroInput {
    prefix: Ident,
    tag: String,
    operation: Option<Ident>,
    processed: Expr,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let prefix = input.parse::<Ident>()?;
        input.parse::<Token![,]>()?;
        let tag = input.parse::<Ident>()?.to_string();
        input.parse::<Token![,]>()?;
        let operation = input.parse::<Ident>().ok();
        if operation.is_some() {
            let _ = input.parse::<Token![,]>();
        }
        let processed = input.parse::<Expr>()?;
        Ok(MacroInput {
            prefix,
            tag,
            operation,
            processed,
        })
    }
}

#[proc_macro]
pub fn generate_code_tests(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let macro_input = syn::parse_macro_input!(input as MacroInput);

    let tests = walkdir::WalkDir::new("example-code")
        .into_iter()
        .map(|entry| entry.expect("entry to exist"))
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('_'))
        .map(|entry| entry.path().to_path_buf())
        .map(|path| (module_hierarchy(&path), generate_test(path, &macro_input)))
        .collect::<Vec<(Vec<String>, TokenStream)>>();

    let mut modules = HashMap::<String, Vec<TokenStream>>::new();
    for (hierarchy, test) in tests {
        // TODO: Generate nested module hierarchy
        let module_name = hierarchy.join("_");

        if let Some(tests) = modules.get_mut(&module_name) {
            tests.push(test);
            continue;
        } else {
            modules.insert(module_name, vec![test]);
        }
    }

    modules
        .into_iter()
        .map(|(module_name, tests)| {
            if module_name.is_empty() {
                quote! {
                    #(#tests)*
                }
            } else {
                let module_name = format_ident!("{}", module_name);
                quote! {
                    mod #module_name {
                        use super::*;
                        #(#tests)*
                    }
                }
            }
        })
        .collect::<TokenStream>()
        .into()
}

fn module_hierarchy(path: &Path) -> Vec<String> {
    let relative = &path.strip_prefix("example-code").unwrap();

    let mut hierarchy = relative
        .components()
        .map(|c| c.as_os_str().to_str().unwrap().to_string())
        .collect::<Vec<_>>();
    hierarchy.pop();
    hierarchy
}

fn generate_test(path: PathBuf, macro_input: &MacroInput) -> TokenStream {
    let MacroInput {
        prefix,
        tag,
        operation,
        processed,
    } = macro_input;
    let name = path.file_stem().unwrap().to_str().unwrap();

    let Ok(test_file) = std::fs::read_to_string(&path) else {
        panic!("Test file not found!")
    };

    let Ok(expected_struct) = expected_result(&test_file, tag).map_err(|e| match e {
        TestError::TagNotClosed => panic!("Tag not closed in test file!"),
        TestError::InvalidRustCode => panic!("Invalid Rust code in test file '{}'!", path.to_string_lossy()),
        TestError::TagNotFound => e,
    }) else {
        return quote! {};
    };

    let code = test_file;

    let test_name = format_ident!("{prefix}_{name}");
    if let Some(operation) = operation {
        quote! {
            #[test]
            fn #test_name() {
                let code = #code;
                let expected_struct = #expected_struct.#operation();
                let actual_struct = #processed.#operation();
                assert_eq!(expected_struct, actual_struct);
            }
        }
    } else {
        quote! {
            #[test]
            fn #test_name() {
                let code = #code;
                let expected_struct = #expected_struct;
                let actual_struct = #processed;
                assert_eq!(expected_struct, actual_struct);
            }
        }
    }
}

enum TestError {
    TagNotFound,
    TagNotClosed,
    InvalidRustCode,
}

fn expected_result(test_file: &str, tag: &str) -> std::result::Result<TokenStream, TestError> {
    let prefix = "//@";
    let mut lines = vec![];
    let mut iter = test_file.lines();

    loop {
        match iter.next() {
            Some(line)
                if line
                    .strip_prefix(prefix)
                    .is_some_and(|s| s.trim().starts_with(tag)) =>
            {
                break
            }
            Some(_) => continue,
            None => return Err(TestError::TagNotFound),
        }
    }

    loop {
        match iter.next() {
            Some(line) if line.starts_with("//@end") => break,
            Some(line) => lines.push(line.strip_prefix("//").expect(&format!("line to start with '//': {line} in \n{test_file}"))),
            None => return Err(TestError::TagNotClosed),
        }
    }

    let code = lines.join("\n");
    code.parse::<TokenStream>()
        .map_err(|_| TestError::InvalidRustCode)
}
