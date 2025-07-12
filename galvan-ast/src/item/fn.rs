use galvan_ast_macro::AstNode;

use crate::{AstNode, PrintAst, Span};

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct FnDecl {
    // pub annotations,
    pub signature: FnSignature,
    pub body: Body,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
pub struct FnSignature {
    // pub asyncness: Async,
    // pub constness: Const,
    pub visibility: Visibility,
    pub identifier: Ident,
    pub parameters: ParamList,
    pub return_type: Option<TypeElement>,
    pub span: Span,
}

impl FnSignature {
    pub fn receiver(&self) -> Option<&Param> {
        self.parameters
            .params
            .first()
            .filter(|param| param.identifier.as_str() == "self")
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
