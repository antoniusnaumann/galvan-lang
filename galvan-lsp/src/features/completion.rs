//! `textDocument/completion`.
//!
//! Suggestions are filtered by the syntactic context at the cursor
//! ([`context_at`]), so only names that can actually occur at the position
//! are offered:
//!
//! - **Member completion** after a `.`: the receiver expression's inferred
//!   type is looked up in the HIR ([`galvan_hir::query::expression_at`]), and
//!   the type's fields plus the methods declared on it are offered. When the
//!   cursor sits directly behind the dot (where the file cannot parse yet),
//!   the analysis runs on a *probe* of the document with a placeholder
//!   identifier inserted after the dot.
//! - **Path completion** after `::`: in Galvan `::` only occurs in paths
//!   (package qualifiers and enum cases), so the cases of the enum named
//!   before the `::` are offered — and nothing else. Unknown qualifiers
//!   (e.g. external packages) yield no suggestions rather than noise.
//! - **Type completion** where a type is expected (after `->`, or after a
//!   `:` that annotates a binding, parameter or field): type names only.
//! - **Nothing** where a *new* name is being introduced (after `let`, `fn`,
//!   `type`, `for`, or a parameter name in a signature).
//! - **Value completion** everywhere else: local bindings and parameters in
//!   scope at the cursor (from the typechecker's symbol index), top-level
//!   functions and types from every file of the crate, and the keywords
//!   valid at the position. Items are ranked locals first, then functions,
//!   types and keywords.

use std::collections::HashSet;
use std::path::Path;

use galvan_ast::{TypeDecl, TypeElement};
use galvan_hir::{query, DefinitionKind, SymbolIndex};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position};

use crate::analysis::render_definition;
use crate::document::Document;
use crate::workspace::{Analysis, Crate};

/// Keywords that introduce a top-level declaration.
const TOPLEVEL_KEYWORDS: &[&str] = &["fn", "type", "test", "main", "pub", "use", "async", "build", "cmd"];
/// Keywords that can start a statement inside a body.
const STATEMENT_KEYWORDS: &[&str] = &[
    "let", "mut", "ref", "if", "else", "while", "for", "return", "match", "break", "continue",
];
/// Keywords that can occur inside an expression.
const EXPRESSION_KEYWORDS: &[&str] = &["true", "false", "and", "or", "not", "if", "match"];

/// Sort-group prefixes: clients order completions by `sort_text`, so items
/// are ranked locals < functions < types < keywords within a response.
const SORT_LOCAL: char = '0';
const SORT_FUNCTION: char = '1';
const SORT_TYPE: char = '2';
const SORT_KEYWORD: char = '3';

pub fn completion(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
) -> Vec<CompletionItem> {
    let offset = current.line_index.offset(&current.text, position);

    if let (Some(offset), Some(file)) = (offset, file) {
        match context_at(&current.text, offset) {
            Context::Member { dot } => return member_completion(current, krate, file, dot),
            Context::Path {
                qualifier,
                ident_start,
            } => return path_completion(current, krate, file, ident_start, &qualifier),
            Context::Type => return type_completion(krate),
            Context::NewName => return Vec::new(),
            Context::Value { word_start } => {
                if let Some(analysis) = krate.analyze() {
                    return value_completion(
                        &analysis.index,
                        file,
                        offset,
                        &current.text,
                        word_start,
                    );
                }
            }
        }
    }

    // The crate does not typecheck (or the document has no file): offer
    // top-level declarations and keywords from whatever parses.
    let mut items = ast_function_items(krate);
    items.extend(ast_type_items(krate));
    items.extend(keyword_items(
        TOPLEVEL_KEYWORDS
            .iter()
            .chain(STATEMENT_KEYWORDS)
            .chain(EXPRESSION_KEYWORDS)
            .copied(),
    ));
    items
}

/// The syntactic context the cursor completes in.
enum Context {
    /// After a `.`, at the given byte offset.
    Member { dot: usize },
    /// After `qualifier::`; `ident_start` is where the (partial) case name
    /// begins.
    Path {
        qualifier: String,
        ident_start: usize,
    },
    /// A type is expected (after `->` or an annotating `:`).
    Type,
    /// A new name is being introduced; nothing sensible to suggest.
    NewName,
    /// An expression or statement; `word_start` is where the (partial)
    /// identifier under the cursor begins.
    Value { word_start: usize },
}

fn context_at(text: &str, offset: usize) -> Context {
    let bytes = text.as_bytes();

    // Start of the (partial) identifier the cursor sits in or behind.
    let mut ident_start = offset.min(bytes.len());
    while ident_start > 0 && is_ident_byte(bytes[ident_start - 1]) {
        ident_start -= 1;
    }

    if ident_start > 0 && bytes[ident_start - 1] == b'.' {
        return Context::Member {
            dot: ident_start - 1,
        };
    }
    if ident_start >= 2 && bytes[ident_start - 1] == b':' && bytes[ident_start - 2] == b':' {
        let end = ident_start - 2;
        let mut start = end;
        while start > 0 && is_ident_byte(bytes[start - 1]) {
            start -= 1;
        }
        return Context::Path {
            qualifier: text[start..end].to_string(),
            ident_start,
        };
    }

    // The token preceding the identifier (horizontal whitespace skipped).
    let mut prev = ident_start;
    while prev > 0 && matches!(bytes[prev - 1], b' ' | b'\t') {
        prev -= 1;
    }

    if prev >= 2 && bytes[prev - 1] == b'>' && bytes[prev - 2] == b'-' {
        return Context::Type;
    }
    if prev > 0 && bytes[prev - 1] == b':' && (prev < 2 || bytes[prev - 2] != b':') {
        return if colon_introduces_type(text, prev - 1) {
            Context::Type
        } else {
            // A call-argument label or dict entry: the value is an expression.
            Context::Value { word_start: ident_start }
        };
    }

    // A declaration keyword introduces a fresh name...
    let mut word_start = prev;
    while word_start > 0 && is_ident_byte(bytes[word_start - 1]) {
        word_start -= 1;
    }
    if matches!(
        &text[word_start..prev],
        "let" | "mut" | "ref" | "fn" | "type" | "for"
    ) {
        return Context::NewName;
    }
    // ...and so does a parameter position in a signature: `fn f(<here>`.
    if matches!(prev.checked_sub(1).map(|i| bytes[i]), Some(b'(') | Some(b','))
        && in_signature_parens(text, ident_start)
    {
        return Context::NewName;
    }

    Context::Value { word_start: ident_start }
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Whether the `:` at byte `colon` annotates a name with its type (binding
/// annotation, function parameter or struct field) rather than labelling a
/// call argument or dict entry.
fn colon_introduces_type(text: &str, colon: usize) -> bool {
    let bytes = text.as_bytes();

    // The name before the colon.
    let mut name_end = colon;
    while name_end > 0 && matches!(bytes[name_end - 1], b' ' | b'\t') {
        name_end -= 1;
    }
    let mut name_start = name_end;
    while name_start > 0 && is_ident_byte(bytes[name_start - 1]) {
        name_start -= 1;
    }
    if name_start == name_end {
        return false;
    }

    // `let name:` / `mut name:` / `ref name:` annotate a binding.
    let mut prev = name_start;
    while prev > 0 && matches!(bytes[prev - 1], b' ' | b'\t') {
        prev -= 1;
    }
    let mut word_start = prev;
    while word_start > 0 && is_ident_byte(bytes[word_start - 1]) {
        word_start -= 1;
    }
    if matches!(&text[word_start..prev], "let" | "mut" | "ref") {
        return true;
    }

    // Parameters of a `fn` signature and members of a `type` body are typed;
    // everything else (call-argument labels, dict keys) is not.
    in_signature_parens(text, name_start) || in_type_body(text, name_start)
}

/// Whether `pos` sits inside the parameter parentheses of a `fn` signature
/// (as opposed to the argument list of a call).
fn in_signature_parens(text: &str, pos: usize) -> bool {
    let bytes = text.as_bytes();
    let Some(open) = enclosing_open_paren(bytes, pos) else {
        return false;
    };
    // The function name directly before the `(` and the keyword before that.
    let mut name_end = open;
    while name_end > 0 && matches!(bytes[name_end - 1], b' ' | b'\t') {
        name_end -= 1;
    }
    let mut name_start = name_end;
    while name_start > 0 && is_ident_byte(bytes[name_start - 1]) {
        name_start -= 1;
    }
    if name_start == name_end {
        return false;
    }
    let mut keyword_end = name_start;
    while keyword_end > 0 && matches!(bytes[keyword_end - 1], b' ' | b'\t') {
        keyword_end -= 1;
    }
    let mut keyword_start = keyword_end;
    while keyword_start > 0 && is_ident_byte(bytes[keyword_start - 1]) {
        keyword_start -= 1;
    }
    &text[keyword_start..keyword_end] == "fn"
}

/// Byte offset of the nearest unmatched `(` before `pos`, if any.
fn enclosing_open_paren(bytes: &[u8], pos: usize) -> Option<usize> {
    let mut depth = 0usize;
    for i in (0..pos.min(bytes.len())).rev() {
        match bytes[i] {
            b')' => depth += 1,
            b'(' if depth == 0 => return Some(i),
            b'(' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Whether `pos` is inside the body of a `type` declaration, where a `:`
/// annotates a member with its type.
fn in_type_body(text: &str, pos: usize) -> bool {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    for i in (0..pos.min(bytes.len())).rev() {
        match bytes[i] {
            b'}' => depth += 1,
            b'{' if depth == 0 => {
                let line_start = text[..i].rfind('\n').map_or(0, |newline| newline + 1);
                let header = &text[line_start..i];
                return header.split_whitespace().take(2).any(|word| word == "type");
            }
            b'{' => depth -= 1,
            _ => {}
        }
    }
    false
}

/// Whether `pos` is inside any (unclosed) brace, i.e. inside a body rather
/// than at the top level of the file.
fn inside_braces(text: &str, pos: usize) -> bool {
    let mut depth = 0i32;
    for &byte in &text.as_bytes()[..pos.min(text.len())] {
        match byte {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
    }
    depth > 0
}

/// Whether the word starting at `word_start` is the first token of its
/// statement (only whitespace between it and the previous line or brace).
fn at_statement_start(text: &str, word_start: usize) -> bool {
    let bytes = text.as_bytes();
    let mut i = word_start.min(bytes.len());
    while i > 0 && matches!(bytes[i - 1], b' ' | b'\t') {
        i -= 1;
    }
    i == 0 || matches!(bytes[i - 1], b'\n' | b'{' | b'}')
}

fn item(
    label: String,
    kind: CompletionItemKind,
    detail: Option<String>,
    sort_group: char,
) -> CompletionItem {
    CompletionItem {
        sort_text: Some(format!("{sort_group}_{label}")),
        label,
        kind: Some(kind),
        detail,
        ..Default::default()
    }
}

fn keyword_items(keywords: impl IntoIterator<Item = &'static str>) -> Vec<CompletionItem> {
    keywords
        .into_iter()
        .map(|keyword| item(keyword.to_string(), CompletionItemKind::KEYWORD, None, SORT_KEYWORD))
        .collect()
}

/// Completion of the members (fields and methods) of the receiver before the
/// dot at byte offset `dot`.
fn member_completion(
    current: &Document,
    krate: &Crate,
    file: &Path,
    dot: usize,
) -> Vec<CompletionItem> {
    if let Some(items) = krate
        .analyze()
        .and_then(|analysis| member_items(analysis, file, dot))
    {
        return items;
    }

    // The receiver was not found: the document does not parse as-is (e.g.
    // the cursor sits directly behind the dot) and is absent from the
    // analysis. Retry on a probe with a placeholder member name.
    let Some(probe) = insert_placeholder(&current.text, dot + 1, 'x') else {
        return Vec::new();
    };
    let probe_crate = krate.with_file_text(file, &probe);
    probe_crate
        .analyze()
        .and_then(|analysis| member_items(analysis, file, dot))
        .unwrap_or_default()
}

/// The members of the receiver expression ending at `dot`, or `None` when the
/// receiver cannot be found (as opposed to a receiver without members).
fn member_items(analysis: &Analysis, file: &Path, dot: usize) -> Option<Vec<CompletionItem>> {
    // The receiver is the innermost expression ending at the dot.
    let receiver = query::expression_at(&analysis.module, file, dot.saturating_sub(1))?;
    let receiver_type = receiver_type_name(&receiver.ty)?;

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
        items.push(item(
            definition.name.clone(),
            kind,
            Some(render_definition(definition)),
            SORT_LOCAL,
        ));
    }
    Some(items)
}

/// `text` with `placeholder` inserted at byte `offset`, or `None` if the
/// offset is not a character boundary (never the case for offsets derived
/// from ASCII tokens like `.` and `::`, but guarded to keep probes panic-free).
fn insert_placeholder(text: &str, offset: usize, placeholder: char) -> Option<String> {
    if !text.is_char_boundary(offset) {
        return None;
    }
    let mut probe = text.to_string();
    probe.insert(offset, placeholder);
    Some(probe)
}

/// The named type members are looked up on, if the receiver has one.
fn receiver_type_name(ty: &TypeElement) -> Option<&str> {
    match ty {
        TypeElement::Plain(basic) => Some(basic.ident.as_str()),
        TypeElement::Parametric(parametric) => Some(parametric.base_type.as_str()),
        _ => None,
    }
}

/// Completion after `qualifier::` — the cases of the enum named `qualifier`.
/// Unknown qualifiers (e.g. package names) yield no suggestions.
fn path_completion(
    current: &Document,
    krate: &Crate,
    file: &Path,
    ident_start: usize,
    qualifier: &str,
) -> Vec<CompletionItem> {
    if let Some(analysis) = krate.analyze() {
        let items = variant_items(&analysis.index, qualifier);
        if !items.is_empty() {
            return items;
        }
    }
    // A dangling `Enum::` can keep the file from parsing (dropping it from
    // the analysis); retry with a placeholder case name (enum cases are
    // type identifiers, hence uppercase) at the cursor.
    if let Some(probe) = insert_placeholder(&current.text, ident_start, 'X') {
        let probe_crate = krate.with_file_text(file, &probe);
        if let Some(analysis) = probe_crate.analyze() {
            let items = variant_items(&analysis.index, qualifier);
            if !items.is_empty() {
                return items;
            }
        }
    }
    variant_items_from_asts(krate, qualifier)
}

fn variant_items(index: &SymbolIndex, qualifier: &str) -> Vec<CompletionItem> {
    index
        .definitions()
        .filter_map(|(_, definition)| match &definition.kind {
            DefinitionKind::EnumVariant { owner } if owner.as_str() == qualifier => Some(item(
                definition.name.clone(),
                CompletionItemKind::ENUM_MEMBER,
                Some(render_definition(definition)),
                SORT_LOCAL,
            )),
            _ => None,
        })
        .collect()
}

/// Enum cases of `qualifier` straight from the parsed ASTs, for when the
/// crate does not typecheck at all.
fn variant_items_from_asts(krate: &Crate, qualifier: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    for file in krate.files() {
        let Some(segmented) = file.segmented.as_ref() else {
            continue;
        };
        for ty in &segmented.types {
            let TypeDecl::Enum(decl) = &ty.item else {
                continue;
            };
            if ty.ident().as_str() != qualifier {
                continue;
            }
            for member in &decl.members {
                items.push(item(
                    member.ident.as_str().to_string(),
                    CompletionItemKind::ENUM_MEMBER,
                    Some(format!("{qualifier}::{}", member.ident.as_str())),
                    SORT_LOCAL,
                ));
            }
        }
    }
    items
}

/// Completion where a type is expected: user-declared types plus the builtin
/// type names.
fn type_completion(krate: &Crate) -> Vec<CompletionItem> {
    let mut items = match krate.analyze() {
        Some(analysis) => {
            let enums = enum_type_names(&analysis.index);
            analysis
                .index
                .definitions()
                .filter(|(_, definition)| matches!(definition.kind, DefinitionKind::Type))
                .map(|(_, definition)| {
                    item(
                        definition.name.clone(),
                        type_kind(&enums, &definition.name),
                        Some(render_definition(definition)),
                        SORT_TYPE,
                    )
                })
                .collect()
        }
        None => ast_type_items(krate),
    };
    items.extend(builtin_type_items());
    items
}

/// Names of all types that have enum variants.
fn enum_type_names(index: &SymbolIndex) -> HashSet<&str> {
    index
        .definitions()
        .filter_map(|(_, definition)| match &definition.kind {
            DefinitionKind::EnumVariant { owner } => Some(owner.as_str()),
            _ => None,
        })
        .collect()
}

fn type_kind(enums: &HashSet<&str>, name: &str) -> CompletionItemKind {
    if enums.contains(name) {
        CompletionItemKind::ENUM
    } else {
        CompletionItemKind::STRUCT
    }
}

fn builtin_type_items() -> Vec<CompletionItem> {
    galvan_hir::builtins::builtins()
        .types
        .keys()
        .filter(|ident| !ident.as_str().starts_with("__"))
        .map(|ident| {
            item(
                ident.as_str().to_string(),
                CompletionItemKind::STRUCT,
                None,
                SORT_TYPE,
            )
        })
        .collect()
}

/// Value (expression/statement) completion: in-scope locals, top-level
/// declarations, and the keywords valid at the position.
fn value_completion(
    index: &SymbolIndex,
    file: &Path,
    offset: usize,
    text: &str,
    word_start: usize,
) -> Vec<CompletionItem> {
    // At the top level of a file only declarations can follow.
    if !inside_braces(text, word_start) {
        return keyword_items(TOPLEVEL_KEYWORDS.iter().copied());
    }

    let mut items = Vec::new();

    for (_, definition) in index.visible_locals(file, offset) {
        items.push(item(
            definition.name.clone(),
            CompletionItemKind::VARIABLE,
            Some(render_definition(definition)),
            SORT_LOCAL,
        ));
    }

    let enums = enum_type_names(index);
    for (_, definition) in index.definitions() {
        let (kind, sort_group) = match &definition.kind {
            // Methods are reached through their receiver, not bare names;
            // enum cases through their `Enum::` qualifier.
            DefinitionKind::Function { receiver: None } => {
                (CompletionItemKind::FUNCTION, SORT_FUNCTION)
            }
            // Types are values too: constructor calls and enum qualifiers.
            DefinitionKind::Type => (type_kind(&enums, &definition.name), SORT_TYPE),
            _ => continue,
        };
        items.push(item(
            definition.name.clone(),
            kind,
            Some(render_definition(definition)),
            sort_group,
        ));
    }

    let mut keywords: Vec<&str> = EXPRESSION_KEYWORDS.to_vec();
    if at_statement_start(text, word_start) {
        keywords.extend_from_slice(STATEMENT_KEYWORDS);
    }
    items.extend(keyword_items(keywords));
    items
}

/// Fallback: top-level functions from every file that parses.
fn ast_function_items(krate: &Crate) -> Vec<CompletionItem> {
    use crate::features::span_text;

    let mut items = Vec::new();
    for file in krate.files() {
        let Some(segmented) = file.segmented.as_ref() else {
            continue;
        };
        let text = file.source.content();

        for func in &segmented.functions {
            items.push(item(
                func.signature.identifier.as_str().to_string(),
                CompletionItemKind::FUNCTION,
                Some(span_text(text, func.signature.span).to_string()),
                SORT_FUNCTION,
            ));
        }
    }
    items
}

/// Fallback: top-level types from every file that parses.
fn ast_type_items(krate: &Crate) -> Vec<CompletionItem> {
    use crate::features::span_text;

    let mut items = Vec::new();
    for file in krate.files() {
        let Some(segmented) = file.segmented.as_ref() else {
            continue;
        };
        let text = file.source.content();

        for ty in &segmented.types {
            let header = span_text(text, ty.item.span());
            let header = header.split('{').next().unwrap_or(header).trim();
            let kind = if matches!(ty.item, TypeDecl::Enum(_)) {
                CompletionItemKind::ENUM
            } else {
                CompletionItemKind::STRUCT
            };
            items.push(item(
                ty.ident().as_str().to_string(),
                kind,
                Some(header.to_string()),
                SORT_TYPE,
            ));
        }
    }
    items
}
