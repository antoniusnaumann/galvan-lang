use crate::{AstNode, PrintAst, Span};
use galvan_ast_macro::AstNode;
use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visibility {
    pub kind: VisibilityKind,
    span: Span,
}

impl Visibility {
    pub fn new(kind: VisibilityKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl Deref for Visibility {
    type Target = VisibilityKind;

    fn deref(&self) -> &VisibilityKind {
        &self.kind
    }
}

impl AstNode for Visibility {
    fn span(&self) -> &Span {
        &self.span
    }

    fn print(&self, indent: usize) -> String {
        let stringified = match self.kind {
            VisibilityKind::Inherited => "inherited".to_string(),
            VisibilityKind::Public => "pub".to_string(),
            VisibilityKind::Private => "private".to_string(),
        };

        format!("{}{}", " ".repeat(indent), stringified)
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum VisibilityKind {
    // Inherited usually means pub(crate)
    #[default]
    Inherited,
    Public,
    Private,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ownership {
    Owned,
    Borrowed,
    MutBorrowed,
    Copy,
    Ref,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Async {
    Async,
    // This usually means sync
    #[default]
    Inherited,
    // This is not implemented but will be supported in future versions
    Generic,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Const {
    Const,
    // This usually means not const
    #[default]
    Inherited,
    // This is not implemented but will be supported in future versions
    Generic,
}
