//! Semantic queries built on top of the compiler crates.
//!
//! The primary path uses the typechecker's [`SymbolIndex`]: the compiler
//! records every definition and resolved reference with its source span while
//! typechecking (see `galvan_hir::index`), so mapping a cursor position to a
//! symbol is a plain index lookup — locals, parameters, methods, fields, enum
//! variants and types all resolve the same way.
//!
//! When the crate does not typecheck at all (e.g. a parse error), the server
//! falls back to resolving the token under the cursor *by name* against the
//! crate-wide [`LookupContext`], which still handles top-level functions and
//! types.

use std::path::Path;

use galvan_ast::{FnDecl, Ident, Span, ToplevelItem, TypeDecl, TypeIdent};
use galvan_files::Source;
use galvan_hir::{Definition, DefinitionId, DefinitionKind, SymbolIndex};
use galvan_parse::{Node, ParseTree};
use galvan_resolver::{Lookup, LookupContext};

use crate::features::span_text;

/// Where a definition lives: its source and the spans to present or jump to.
pub struct DefinitionSite<'a> {
    pub source: &'a Source,
    /// Span of the defining identifier (preferred navigation target).
    pub target: Span,
}

/// Resolve the symbol at `offset` in `file` through the symbol index.
pub fn symbol_at<'a>(
    index: &'a SymbolIndex,
    file: &Path,
    offset: usize,
) -> Option<(DefinitionId, &'a Definition)> {
    let id = index.symbol_at(file, offset)?;
    Some((id, index.definition(id)))
}

/// The location to navigate to for a definition. `None` for definitions
/// without a source location (builtins).
pub fn definition_site(definition: &Definition) -> Option<DefinitionSite<'_>> {
    let target = if definition.span == Span::default() {
        definition.decl_span
    } else {
        definition.span
    };
    if target == Span::default() {
        return None;
    }
    Some(DefinitionSite {
        source: &definition.source,
        target,
    })
}

/// Render the declaration of `definition` as a line of Galvan code, suitable
/// for hover contents or a completion detail.
pub fn render_definition(definition: &Definition) -> String {
    let declared = span_text(definition.source.content(), definition.decl_span);
    match &definition.kind {
        DefinitionKind::Function { .. } => {
            if declared.is_empty() {
                // Builtins carry no source text.
                format!("fn {}", definition.name)
            } else {
                declared.to_string()
            }
        }
        DefinitionKind::Type => {
            // Show the type header, not the entire body which may be large.
            let header = declared.split('{').next().unwrap_or(declared).trim();
            if header.is_empty() {
                format!("type {}", definition.name)
            } else {
                header.to_string()
            }
        }
        DefinitionKind::Local { modifier, ty, .. } => {
            let modifier = match modifier {
                galvan_ast::DeclModifier::Let => "let",
                galvan_ast::DeclModifier::Mut => "mut",
                galvan_ast::DeclModifier::Ref => "ref",
                galvan_ast::DeclModifier::Move => "move",
            };
            format!("{modifier} {}: {ty}", definition.name)
        }
        DefinitionKind::Parameter { ty, .. } => format!("{}: {ty}", definition.name),
        DefinitionKind::Field { owner, ty } => format!("{owner}.{}: {ty}", definition.name),
        DefinitionKind::EnumVariant { owner } => format!("{owner}::{}", definition.name),
    }
}

// ----------------------------------------------------------------------
// Name-based fallback (used when the crate fails to typecheck)
// ----------------------------------------------------------------------

/// A name token found under the cursor.
#[derive(Clone, Debug)]
pub enum Token {
    /// A value-level identifier, e.g. a function name or variable.
    Value(String),
    /// A type-level identifier.
    Type(String),
}

/// A [`Token`] together with the byte range it occupies in the document.
#[derive(Clone, Debug)]
pub struct TokenAt {
    pub token: Token,
    pub range: (usize, usize),
}

/// The declaration a token resolves to by name.
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
pub fn token_at(tree: &ParseTree, text: &str, offset: usize) -> Option<TokenAt> {
    let node = ident_node_at(tree, offset)?;
    let name = node.utf8_text(text.as_bytes()).ok()?.to_owned();
    let token = match node.kind() {
        "ident" => Token::Value(name),
        "type_ident" => Token::Type(name),
        _ => return None,
    };
    Some(TokenAt {
        token,
        range: (node.start_byte(), node.end_byte()),
    })
}

/// Return the innermost `ident`/`type_ident` node covering `offset`, walking up
/// from the smallest descendant in case the cursor lands on a hidden leaf.
fn ident_node_at(tree: &ParseTree, offset: usize) -> Option<Node<'_>> {
    let mut node = tree.root_node().descendant_for_byte_range(offset, offset)?;
    loop {
        if matches!(node.kind(), "ident" | "type_ident") {
            return Some(node);
        }
        node = node.parent()?;
    }
}

/// Resolve a token to a top-level declaration by name. Only free functions
/// and types can be resolved this way; everything else requires the symbol
/// index.
pub fn resolve<'a>(lookup: &'a LookupContext<'a>, token: &Token) -> Option<Resolved<'a>> {
    match token {
        Token::Type(name) => {
            let decl = lookup.resolve_type(&TypeIdent::new(name.clone()))?;
            Some(Resolved {
                span: decl.item.span(),
                source: &decl.source,
                kind: ResolvedKind::Type(decl),
            })
        }
        Token::Value(name) => {
            let decl = lookup.resolve_function(None, &Ident::new(name.clone()), &[])?;
            Some(Resolved {
                span: decl.signature.span,
                source: &decl.source,
                kind: ResolvedKind::Function(decl),
            })
        }
    }
}

/// The span of a type declaration (kept for callers of the fallback path).
pub fn type_decl_span(decl: &TypeDecl) -> Span {
    decl.span()
}
