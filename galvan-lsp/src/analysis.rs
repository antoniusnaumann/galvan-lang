//! Semantic queries over a [`Document`], built on top of the compiler crates.
//!
//! The strategy is to use the tree-sitter syntax tree to find the *token* under
//! the cursor (the AST does not carry identifier spans — see
//! `compiler-features.md`), then resolve that token's name against the
//! [`LookupContext`] derived from the segmented AST.

use galvan_ast::{FnDecl, Ident, Span, ToplevelItem, TypeDecl, TypeIdent};
use galvan_parse::{Node, ParseTree};
use galvan_resolver::{Lookup, LookupContext};

use crate::document::Document;

/// A name token found under the cursor.
#[derive(Clone, Debug)]
pub enum Symbol {
    /// A value-level identifier, e.g. a function name or variable.
    Value(String),
    /// A type-level identifier.
    Type(String),
}

/// The declaration a symbol resolves to, with the span to navigate to and the
/// text to show on hover.
pub struct Resolved<'a> {
    pub kind: ResolvedKind<'a>,
    /// Span of the whole declaration (used as the navigation target).
    pub span: Span,
}

pub enum ResolvedKind<'a> {
    Function(&'a ToplevelItem<FnDecl>),
    Type(&'a ToplevelItem<TypeDecl>),
}

/// Find the identifier or type-identifier token at the given byte offset.
pub fn symbol_at(tree: &ParseTree, text: &str, offset: usize) -> Option<Symbol> {
    let node = node_at(tree, offset)?;
    let name = node.utf8_text(text.as_bytes()).ok()?.to_owned();
    match node.kind() {
        "ident" => Some(Symbol::Value(name)),
        "type_ident" => Some(Symbol::Type(name)),
        _ => None,
    }
}

/// Return the innermost `ident`/`type_ident` node covering `offset`, walking up
/// from the smallest descendant in case the cursor lands on a hidden leaf.
fn node_at(tree: &ParseTree, offset: usize) -> Option<Node<'_>> {
    let mut node = tree
        .root_node()
        .descendant_for_byte_range(offset, offset)?;
    loop {
        if matches!(node.kind(), "ident" | "type_ident") {
            return Some(node);
        }
        node = node.parent()?;
    }
}

/// Resolve a symbol to its declaration within the same document.
///
/// Resolution is limited to top-level functions (without receiver/labels) and
/// types — the cases the [`LookupContext`] supports without type inference.
/// See `compiler-features.md` for what is not yet possible.
pub fn resolve<'a>(lookup: &'a LookupContext<'a>, symbol: &Symbol) -> Option<Resolved<'a>> {
    match symbol {
        Symbol::Type(name) => {
            let decl = lookup.resolve_type(&TypeIdent::new(name.clone()))?;
            Some(Resolved {
                span: type_decl_span(decl),
                kind: ResolvedKind::Type(decl),
            })
        }
        Symbol::Value(name) => {
            let decl = lookup.resolve_function(None, &Ident::new(name.clone()), &[])?;
            Some(Resolved {
                span: decl.signature.span,
                kind: ResolvedKind::Function(decl),
            })
        }
    }
}

/// Build a best-effort lookup context for a document. Duplicate declarations
/// (which are themselves diagnostics) are tolerated: resolution stays partial
/// rather than failing outright.
pub fn lookup_context(document: &Document) -> Option<LookupContext<'_>> {
    let segmented = document.segmented.as_ref()?;
    let mut lookup = LookupContext::new();
    let _ = lookup.add_from(segmented);
    Some(lookup)
}

pub fn type_decl_span(decl: &TypeDecl) -> Span {
    match decl {
        TypeDecl::Tuple(t) => t.span,
        TypeDecl::Struct(s) => s.span,
        TypeDecl::Alias(a) => a.span,
        TypeDecl::Enum(e) => e.span,
        TypeDecl::Empty(e) => e.span,
    }
}
