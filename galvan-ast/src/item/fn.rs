use galvan_ast_macro::AstNode;

use crate::{AstNode, Ident, PrintAst, Span, TypeIdent};

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct WhereBound {
    pub type_params: Vec<Ident>,
    pub bounds: Vec<TypeIdent>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct WhereClause {
    pub bounds: Vec<WhereBound>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct FnDecl {
    // pub annotations,
    pub signature: FnSignature,
    pub body: Body,
    pub span: Span,
}

impl From<FnSignature> for FnDecl {
    fn from(value: FnSignature) -> Self {
        Self {
            signature: value,
            body: Body {
                statements: vec![],
                span: Span::default(),
            },
            span: Span::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct FnSignature {
    // pub asyncness: Async,
    // pub constness: Const,
    pub visibility: Visibility,
    pub identifier: Ident,
    pub parameters: ParamList,
    pub return_type: TypeElement,
    pub where_clause: Option<WhereClause>,
    pub span: Span,
}

impl FnSignature {
    pub fn receiver(&self) -> Option<&Param> {
        self.parameters
            .params
            .first()
            .filter(|param| param.identifier.as_str() == "self")
    }

    /// Collect all generic type parameters from this function signature
    pub fn collect_generics(&self) -> std::collections::HashSet<Ident> {
        let mut generics = std::collections::HashSet::new();

        // Collect from parameters
        for param in &self.parameters.params {
            param.param_type.collect_generics_recursive(&mut generics);
        }

        // Collect from return type
        self.return_type.collect_generics_recursive(&mut generics);

        generics
    }
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct ParamList {
    pub params: Vec<Param>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct Param {
    pub decl_modifier: Option<DeclModifier>,
    pub identifier: Ident,
    pub param_type: TypeElement,
    pub span: Span,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DeclModifier {
    Let,
    Mut,
    Ref,
}

impl PrintAst for DeclModifier {
    fn print_ast(&self, indent: usize) -> String {
        let indent_str = " ".repeat(indent);
        match self {
            DeclModifier::Let => format!("{indent_str}let"),
            DeclModifier::Mut => format!("{indent_str}mut"),
            DeclModifier::Ref => format!("{indent_str}ref"),
        }
    }
}
