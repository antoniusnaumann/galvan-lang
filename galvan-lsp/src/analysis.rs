//! Semantic queries built on top of the compiler crates.
//!
//! The strategy is to use the tree-sitter syntax tree to find the *token* under
//! the cursor (the AST does not carry identifier spans — see
//! `compiler-features.md`), then resolve that token's name against a
//! [`LookupContext`]. The lookup is crate-wide (see [`crate::workspace`]), so
//! resolution crosses file boundaries within a crate.

use galvan_ast::{FnDecl, Ident, Span, ToplevelItem, TypeDecl, TypeIdent};
use galvan_files::Source;
use galvan_parse::{Node, ParseTree};
use galvan_resolver::{Lookup, LookupContext};

/// A name token found under the cursor.
#[derive(Clone, Debug)]
pub enum Symbol {
    /// A value-level identifier, e.g. a function name or variable.
    Value(String),
    /// A type-level identifier.
    Type(String),
}

/// A [`Symbol`] together with the byte range it occupies in the document it was
/// found in (used to highlight the hovered token).
#[derive(Clone, Debug)]
pub struct SymbolToken {
    pub symbol: Symbol,
    pub range: (usize, usize),
}

/// The declaration a symbol resolves to.
pub struct Resolved<'a> {
    pub kind: ResolvedKind<'a>,
    /// Span of the whole declaration (used as the navigation target).
    pub span: Span,
    /// The source file the declaration lives in.
    pub source: &'a Source,
}

pub enum ResolvedKind<'a> {
    Function(&'a ToplevelItem<FnDecl>),
    Type(&'a ToplevelItem<TypeDecl>),
}

/// Find the identifier or type-identifier token at the given byte offset.
pub fn symbol_at(tree: &ParseTree, text: &str, offset: usize) -> Option<SymbolToken> {
    let node = node_at(tree, offset)?;
    let name = node.utf8_text(text.as_bytes()).ok()?.to_owned();
    let symbol = match node.kind() {
        "ident" => Symbol::Value(name),
        "type_ident" => Symbol::Type(name),
        _ => return None,
    };
    Some(SymbolToken {
        symbol,
        range: (node.start_byte(), node.end_byte()),
    })
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

/// Resolve a symbol to its declaration.
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
                source: &decl.source,
                kind: ResolvedKind::Type(decl),
            })
        }
        Symbol::Value(name) => {
            let decl = lookup.resolve_function(None, &Ident::new(name.clone()), &[])?;
            Some(Resolved {
                span: decl.signature.span,
                source: &decl.source,
                kind: ResolvedKind::Function(decl),
            })
        }
    }
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
