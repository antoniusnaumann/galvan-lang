use std::collections::HashMap;

use galvan_ast::{Ast, FnDecl, MainDecl, RootItem, TypeDecl};

pub struct LookupContext<'a> {
    /// Types are resolved by their name
    pub types: HashMap<String, &'a TypeDecl>,
    /// Functions are resolved by their name and - if present - named arguments and their receiver type
    ///
    /// `fn foo(a: i32, b: i32) -> i32` is identified as `foo`
    /// `fn foo(bar a: i32, b: i32) -> i32` is identified as `foo:bar`
    /// `fn foo(self: i32, b: i32) -> i32` is identified as `i32::foo`
    pub functions: HashMap<String, &'a FnDecl>,
    // TODO: Nested contexts for resolving names from imported modules
    // pub imports: HashMap<String, LookupContext<'a>>,
    pub main: Option<&'a MainDecl>,
}

// TODO: derive thiserror and add proper error handling #[derive(Error)]
// TODO: Include spans in errors
pub enum LookupError {
    TypeNotFound,
    FunctionNotFound,
    DuplicateMain,
    DuplicateType,
    DuplicateFunction,
}

impl<'a> TryFrom<&'a [Ast]> for LookupContext<'a> {
    type Error = LookupError;

    fn try_from(asts: &'a [Ast]) -> Result<Self, Self::Error> {
        let mut types = HashMap::new();
        let mut functions = HashMap::new();
        let mut main = None;

        for ast in asts {
            for top in &ast.toplevel {
                match top {
                    RootItem::Type(type_decl) => {
                        types.insert(type_decl.ident().to_string(), type_decl);
                    }
                    RootItem::Fn(fn_decl) => {
                        // TODO: Add named arguments and receiver type
                        functions.insert(fn_decl.signature.identifier.to_string(), fn_decl);
                    }
                    RootItem::Test(_) => {}
                    RootItem::Main(m) => {
                        if main.is_some() {
                            return Err(LookupError::DuplicateMain);
                        }
                        main = Some(m);
                    }
                }
            }
        }

        Ok(LookupContext {
            types,
            functions,
            main,
        })
    }
}
