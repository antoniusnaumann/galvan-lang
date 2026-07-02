//! `textDocument/hover`.
//!
//! Resolution order:
//! 1. the symbol index (locals, parameters, functions, methods, types,
//!    fields, enum variants — wherever the cursor is on an identifier),
//! 2. the inferred type of the innermost expression under the cursor,
//! 3. name-based lookup of top-level declarations (only when the crate does
//!    not typecheck at all).

use std::path::Path;

use galvan_hir::query;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

use crate::analysis::{self, ResolvedKind};
use crate::document::Document;
use crate::features::{doc_comment, span_text};
use crate::workspace::{Analysis, Crate};

pub fn hover(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
) -> Option<Hover> {
    let offset = current.line_index.offset(&current.text, position)?;

    if let (Some(analysis), Some(file)) = (krate.analyze(), file) {
        if let Some(hover) = index_hover(current, &analysis, file, offset) {
            return Some(hover);
        }
        if let Some(hover) = expression_hover(current, &analysis, file, offset) {
            return Some(hover);
        }
        return None;
    }

    fallback_hover(current, krate, offset)
}

/// Hover for an identifier resolved through the symbol index.
fn index_hover(
    current: &Document,
    analysis: &Analysis,
    file: &Path,
    offset: usize,
) -> Option<Hover> {
    let (_, definition) = analysis::symbol_at(&analysis.index, file, offset)?;

    let mut markdown = format!("```galvan\n{}\n```", analysis::render_definition(definition));
    if let Some(doc) = doc_comment(definition.source.content(), definition.span) {
        markdown.push_str("\n\n");
        markdown.push_str(&doc);
    }

    Some(markdown_hover(markdown, None, current))
}

/// Hover showing the inferred type of the expression under the cursor.
fn expression_hover(
    current: &Document,
    analysis: &Analysis,
    file: &Path,
    offset: usize,
) -> Option<Hover> {
    let expression = query::expression_at(&analysis.module, file, offset)?;
    if matches!(
        expression.ty,
        galvan_ast::TypeElement::Infer(_) | galvan_ast::TypeElement::Void(_)
    ) {
        return None;
    }
    let range = current
        .line_index
        .range(&current.text, expression.span);
    Some(markdown_hover(
        format!("```galvan\n{}\n```", expression.ty),
        Some(range),
        current,
    ))
}

/// Name-based hover used when the crate cannot be typechecked.
fn fallback_hover(current: &Document, krate: &Crate, offset: usize) -> Option<Hover> {
    let tree = current.tree.as_ref()?;
    let token = analysis::token_at(tree, &current.text, offset)?;

    let lookup = krate.lookup();
    let resolved = analysis::resolve(&lookup, &token.token)?;

    // The declaration may live in another file, so format from its own source.
    let target = resolved.source.content();
    let signature = match resolved.kind {
        ResolvedKind::Function(func) => span_text(target, func.signature.span).to_string(),
        ResolvedKind::Type(decl) => {
            // Show the type header, not the entire body which may be large.
            let full = span_text(target, decl.item.span());
            full.split('{').next().unwrap_or(full).trim().to_string()
        }
    };

    let mut markdown = format!("```galvan\n{signature}\n```");
    if let Some(doc) = doc_comment(target, resolved.span) {
        markdown.push_str("\n\n");
        markdown.push_str(&doc);
    }

    let range = current
        .line_index
        .byte_range(&current.text, token.range.0, token.range.1);
    Some(markdown_hover(markdown, Some(range), current))
}

fn markdown_hover(markdown: String, range: Option<Range>, _current: &Document) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range,
    }
}
