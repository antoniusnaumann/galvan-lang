//! `textDocument/completion`.
//!
//! Two modes:
//!
//! - **Member completion** after a `.`: the receiver expression's inferred
//!   type is looked up in the HIR ([`galvan_hir::query::expression_at`]), and
//!   the type's fields plus the methods declared on it are offered. When the
//!   cursor sits directly behind the dot (where the file cannot parse yet),
//!   the analysis runs on a *probe* of the document with a placeholder
//!   identifier inserted after the dot.
//! - **Identifier completion** everywhere else: local bindings and parameters
//!   in scope at the cursor (from the typechecker's symbol index), top-level
//!   functions and types from every file of the crate, and keywords.

use std::path::Path;

use galvan_ast::TypeElement;
use galvan_hir::{query, DefinitionKind};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position};

use crate::analysis::render_definition;
use crate::document::Document;
use crate::workspace::{Analysis, Crate};

/// Galvan keywords offered unconditionally.
const KEYWORDS: &[&str] = &[
    "fn", "type", "test", "main", "let", "mut", "ref", "if", "else", "while", "for", "in",
    "return", "match", "true", "false", "and", "or", "not", "use", "pub", "struct", "enum",
    "break", "continue", "async", "build", "cmd",
];

pub fn completion(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
) -> Vec<CompletionItem> {
    let offset = current.line_index.offset(&current.text, position);

    if let (Some(offset), Some(file)) = (offset, file) {
        if let Some(dot) = member_access_at(&current.text, offset) {
            return member_completion(current, krate, file, dot);
        }
        if let Some(analysis) = krate.analyze() {
            return identifier_completion(&analysis, file, offset);
        }
    }

    // The crate does not typecheck (or the document has no file): offer
    // top-level declarations and keywords from whatever parses.
    let mut items = toplevel_completions_from_asts(krate);
    items.extend(keyword_completions());
    items
}

/// Byte offset of the `.` the cursor completes after, if any: the cursor may
/// be directly behind the dot or within a partial identifier following it.
fn member_access_at(text: &str, offset: usize) -> Option<usize> {
    let mut cursor = offset;
    let bytes = text.as_bytes();
    while cursor > 0 && is_ident_byte(bytes[cursor - 1]) {
        cursor -= 1;
    }
    (cursor > 0 && bytes[cursor - 1] == b'.').then(|| cursor - 1)
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Completion of the members (fields and methods) of the receiver before the
/// dot at byte offset `dot`.
fn member_completion(
    current: &Document,
    krate: &Crate,
    file: &Path,
    dot: usize,
) -> Vec<CompletionItem> {
    // If the document does not parse as-is (e.g. the cursor sits directly
    // behind the dot), analyze a probe with a placeholder member name.
    let analysis = krate.analyze().or_else(|| {
        let mut probe = current.text.clone();
        probe.insert(dot + 1, 'x');
        krate.with_file_text(file, &probe).analyze()
    });
    let Some(analysis) = analysis else {
        return Vec::new();
    };

    // The receiver is the innermost expression ending at the dot.
    let Some(receiver) = query::expression_at(&analysis.module, file, dot.saturating_sub(1)) else {
        return Vec::new();
    };
    let Some(receiver_type) = receiver_type_name(&receiver.ty) else {
        return Vec::new();
    };

    let mut items = Vec::new();
    for (_, definition) in analysis.index.definitions() {
        let (kind, owner) = match &definition.kind {
            DefinitionKind::Field { owner, .. } => (CompletionItemKind::FIELD, owner),
            DefinitionKind::Function {
                receiver: Some(owner),
            } => (CompletionItemKind::METHOD, owner),
            _ => continue,
        };
        if owner.as_str() != receiver_type {
            continue;
        }
        items.push(CompletionItem {
            label: definition.name.clone(),
            kind: Some(kind),
            detail: Some(render_definition(definition)),
            ..Default::default()
        });
    }
    items
}

/// The named type members are looked up on, if the receiver has one.
fn receiver_type_name(ty: &TypeElement) -> Option<&str> {
    match ty {
        TypeElement::Plain(basic) => Some(basic.ident.as_str()),
        TypeElement::Parametric(parametric) => Some(parametric.base_type.as_str()),
        _ => None,
    }
}

/// Identifier completion: in-scope locals, top-level declarations, keywords.
fn identifier_completion(analysis: &Analysis, file: &Path, offset: usize) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for (_, definition) in analysis.index.visible_locals(file, offset) {
        items.push(CompletionItem {
            label: definition.name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some(render_definition(definition)),
            ..Default::default()
        });
    }

    for (_, definition) in analysis.index.definitions() {
        let kind = match &definition.kind {
            // Methods are reached through their receiver, not bare names.
            DefinitionKind::Function { receiver: None } => CompletionItemKind::FUNCTION,
            DefinitionKind::Type => CompletionItemKind::STRUCT,
            _ => continue,
        };
        items.push(CompletionItem {
            label: definition.name.clone(),
            kind: Some(kind),
            detail: Some(render_definition(definition)),
            ..Default::default()
        });
    }

    items.extend(keyword_completions());
    items
}

fn keyword_completions() -> impl Iterator<Item = CompletionItem> {
    KEYWORDS.iter().map(|keyword| CompletionItem {
        label: keyword.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        ..Default::default()
    })
}

/// Fallback: top-level declarations from every file that parses.
fn toplevel_completions_from_asts(krate: &Crate) -> Vec<CompletionItem> {
    use crate::features::span_text;

    let mut items = Vec::new();
    for file in krate.files() {
        let Some(segmented) = file.segmented.as_ref() else {
            continue;
        };
        let text = file.source.content();

        for func in &segmented.functions {
            items.push(CompletionItem {
                label: func.signature.identifier.as_str().to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(span_text(text, func.signature.span).to_string()),
                ..Default::default()
            });
        }

        for ty in &segmented.types {
            let header = span_text(text, ty.item.span());
            let header = header.split('{').next().unwrap_or(header).trim();
            items.push(CompletionItem {
                label: ty.ident().as_str().to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some(header.to_string()),
                ..Default::default()
            });
        }
    }
    items
}
