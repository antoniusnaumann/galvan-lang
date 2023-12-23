use std::collections::HashMap;

use galvan_ast::{Ast, FnDecl, Ident, MainDecl, RootItem, TypeDecl, TypeIdent};

pub struct LookupContext<'a> {
    /// Types are resolved by their name
    pub types: HashMap<TypeId, &'a TypeDecl>,
    /// Functions are resolved by their name and - if present - named arguments and their receiver type
    ///
    /// `fn foo(a: i32, b: i32) -> i32` is identified as `foo`
    /// `fn foo(bar a: i32, b: i32) -> i32` is identified as `foo:bar`
    /// `fn foo(self: i32, b: i32) -> i32` is identified as `i32::foo`
    pub functions: HashMap<FunctionId, &'a FnDecl>,
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
                        types.insert(type_decl.ident().into(), type_decl);
                    }
                    RootItem::Fn(fn_decl) => {
                        // TODO: Add named arguments and receiver type
                        let func_id = FunctionId::new(None, &fn_decl.signature.identifier, &[]);
                        functions.insert(func_id, fn_decl);
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

impl LookupContext<'_> {
    pub fn resolve_type(&self, name: &TypeIdent) -> Option<&TypeDecl> {
        self.types.get(&name.into()).copied()
    }

    pub fn resolve_function(
        &self,
        receiver: Option<&TypeIdent>,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&FnDecl> {
        let func_id = FunctionId::new(receiver, name, labels);
        self.functions.get(&func_id).copied()
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct TypeId(Box<str>);

impl<S> From<S> for TypeId
where
    S: AsRef<str>,
{
    fn from(ident: S) -> Self {
        Self(ident.as_ref().into())
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct FunctionId(Box<str>);

impl FunctionId {
    fn new(receiver: Option<&TypeIdent>, fn_ident: &Ident, labels: &[&str]) -> Self {
        let mut id = String::new();
        if let Some(receiver) = receiver {
            id.push_str(receiver.as_str());
            id.push_str("::");
        }
        id.push_str(fn_ident.as_str());
        if !labels.is_empty() {
            id.push(':');
            id.push_str(&labels.join(":"));
        }

        Self(id.into())
    }
}
