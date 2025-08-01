use crate::{FunctionId, Lookup, LookupContext};
use galvan_ast::{
    DeclModifier, FnDecl, Ident, Ownership, ToplevelItem, TypeDecl, TypeElement, TypeIdent,
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Scope<'a> {
    pub parent: Option<&'a Scope<'a>>,
    pub variables: HashMap<Ident, Variable>,
    /// expected type of this scopes return value
    pub return_type: TypeElement,
    /// expected ownership of this scopes return value
    pub ownership: Ownership,
    pub fn_return: TypeElement,

    lookup: Option<LookupContext<'a>>,
}

impl Scope<'_> {
    pub fn child(parent: &Self) -> Scope {
        Scope {
            parent: Some(parent),
            variables: HashMap::new(),
            lookup: None,
            return_type: TypeElement::void(),
            ownership: Ownership::UniqueOwned,
            fn_return: parent.fn_return.clone(),
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

impl<'a> Scope<'a> {
    pub fn set_lookup(&mut self, lookup: LookupContext<'a>) {
        self.lookup = Some(lookup);
    }

    pub fn returns(mut self, ty: TypeElement, ownership: Ownership) -> Scope<'a> {
        self.return_type = ty;
        self.ownership = ownership;
        self
    }

    pub fn functions(&self) -> Vec<FunctionId> {
        let mut functions = Vec::new();
        let mut scope = self;

        loop {
            if let Some(ref lookup) = scope.lookup {
                functions.extend(lookup.functions.keys().map(|v| v.to_owned()))
            }

            match scope.parent {
                Some(s) => scope = s,
                None => break,
            }
        }

        functions
    }
}

impl Lookup for Scope<'_> {
    fn resolve_type(&self, name: &TypeIdent) -> Option<&ToplevelItem<TypeDecl>> {
        self.lookup
            .as_ref()
            .and_then(|lookup| lookup.resolve_type(name))
            .or_else(|| self.parent.and_then(|parent| parent.resolve_type(name)))
    }

    fn resolve_function(
        &self,
        receiver: Option<&TypeIdent>,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&ToplevelItem<FnDecl>> {
        self.lookup
            .as_ref()
            .and_then(|lookup| lookup.resolve_function(receiver, name, labels))
            .or_else(|| {
                self.parent
                    .and_then(|parent| parent.resolve_function(receiver, name, labels))
            })
    }
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub ident: Ident,
    pub modifier: DeclModifier,
    pub ty: TypeElement,
    pub ownership: Ownership,
}

impl Variable {
    pub fn is_mut(&self) -> bool {
        matches!(self.modifier, DeclModifier::Mut)
    }
}
