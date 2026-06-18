use std::collections::HashMap;

use galvan_ast::{DeclModifier, Ident, Ownership, TypeElement};

/// A variable visible to the typechecker.
///
/// `ownership` describes how the variable is represented in the generated
/// Rust code (e.g. non-copy `let` parameters are `Borrowed` because they are
/// transpiled to `&T`).
#[derive(Clone, Debug)]
pub struct Variable {
    pub ident: Ident,
    pub modifier: DeclModifier,
    pub ty: TypeElement,
    pub ownership: Ownership,
}

/// Stack of lexical scopes used while lowering a function body
#[derive(Debug, Default)]
pub(crate) struct ScopeStack {
    scopes: Vec<HashMap<Ident, Variable>>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop(&mut self) {
        debug_assert!(self.scopes.len() > 1, "cannot pop the root scope");
        self.scopes.pop();
    }

    pub fn declare(&mut self, variable: Variable) {
        self.scopes
            .last_mut()
            .expect("scope stack is never empty")
            .insert(variable.ident.clone(), variable);
    }

    pub fn get(&self, ident: &Ident) -> Option<&Variable> {
        self.scopes.iter().rev().find_map(|scope| scope.get(ident))
    }

    pub fn variable_names(&self) -> Vec<String> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.keys())
            .map(|ident| ident.to_string())
            .collect()
    }
}
