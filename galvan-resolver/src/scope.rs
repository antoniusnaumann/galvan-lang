use galvan_ast::{DeclModifier, Ident, Ownership, TypeElement};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Scope<'a> {
    pub parent: Option<&'a Scope<'a>>,
    pub variables: HashMap<Ident, Variable>,
}

impl Scope<'_> {
    pub fn child(parent: &Self) -> Scope {
        Scope {
            parent: Some(parent),
            variables: HashMap::new(),
        }
    }

    pub fn declare_variable(&mut self, variable: Variable) {
        self.variables.insert(variable.ident.clone(), variable);
    }

    pub fn get_variable(&self, ident: &Ident) -> Option<&Variable> {
        self.variables
            .get(ident)
            .or_else(|| self.parent.and_then(|parent| parent.get_variable(ident)))
    }
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub ident: Ident,
    pub modifier: DeclModifier,
    /// If the variable type cannot be identified, this is `None` and type inference will be delegated to Rust
    pub ty: Option<TypeElement>,
    pub ownership: Ownership,
}
