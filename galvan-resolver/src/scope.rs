use galvan_ast::{DeclModifier, Ident, Ownership, TypeElement};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Scope<'a> {
    pub parent: Option<&'a Scope<'a>>,
    pub variables: HashMap<Ident, Variable<'a>>,
}

impl Scope<'_> {
    pub fn child(parent: &Self) -> Scope {
        Scope {
            parent: Some(parent),
            variables: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Variable<'a> {
    pub ident: Ident,
    pub modifier: DeclModifier,
    /// If the variable type cannot be identified, this is `None` and type inference will be delegated to Rust
    pub ty: Option<Cow<'a, TypeElement>>,
    pub ownership: Ownership,
}
