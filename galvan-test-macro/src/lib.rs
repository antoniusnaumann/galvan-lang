use std::path::PathBuf;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Token, Result, Expr};

struct MacroInput { prefix: Ident, tag: String, processed: Expr }

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let prefix = input.parse::<Ident>()?;
        input.parse::<Token![,]>()?;
        let tag = input.parse::<Ident>()?.to_string();
        input.parse::<Token![,]>()?;
        let processed = input.parse::<Expr>()?;
        Ok(MacroInput { prefix, tag, processed })
    }
}

#[proc_macro]
pub fn generate_code_tests(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let macro_input = syn::parse_macro_input!(input as MacroInput);

    // panic!("{:?}", std::env::current_dir());

    // Generate the test for each file in the example-code directory
    std::fs::read_dir("example-code")
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().is_ok_and(|ft| ft.is_file()))
        .map(|f| f.path())
        .map(|path| generate_test(path, &macro_input))
        .collect::<TokenStream>()
        .into()
}

fn generate_test(path: PathBuf, macro_input: &MacroInput) -> TokenStream {
    let MacroInput { prefix, tag, processed } = macro_input;
    let name = path.file_stem().unwrap().to_str().unwrap();

    let Ok(test_file) = std::fs::read_to_string(&path) else { panic!("Test file not found!") };

    let split: Vec<_> = test_file.split("---").take(2).map(|s| s.trim()).collect();
    let (frontmatter, code) = (split[0], split[1]);
    let expected_struct = frontmatter
        .lines()
        .filter_map(|line| line.strip_prefix(tag))
        .collect::<Vec<_>>()
        .first()
        .unwrap_or_else(|| panic!("Tag {tag} not found in frontmatter!"))
        .trim()
        .parse::<TokenStream>()
        .unwrap_or_else(|e| panic!("Code in frontmatter is not valid Rust code! Error: {e}\nCode: {code}"));

    let test_name = format_ident!("{prefix}_{name}");
    quote!{
        #[test]
        fn #test_name() {
            let code = #code;
            let expected_struct = #expected_struct;
            let actual_struct = #processed;
            assert_eq!(expected_struct, actual_struct);
        }
    }
}