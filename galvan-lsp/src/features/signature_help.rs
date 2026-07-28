//! `textDocument/signatureHelp`.
//!
//! When the cursor sits inside the argument parentheses of a call, the
//! matching declarations are shown with the argument being typed highlighted:
//!
//! - `name(` — free functions called `name` (all overloads),
//! - `expr.name(` — methods called `name`; when the crate analysis can type
//!   the receiver expression, only methods on that type are offered,
//! - `Type(` — the constructor of struct or tuple type `Type`,
//! - `Enum::Case(` — the fields of that enum case.
//!
//! The call site is found by a forward scan of the document text rather than
//! the parse tree, so help also works while the argument list is still
//! incomplete and the file does not parse. Signature labels are rendered from
//! the declaration's own source text, so what is shown is what the author
//! wrote.
//!
//! The active parameter is the argument the cursor is on (counting commas at
//! the call's own nesting level). A labelled argument (`label: value`)
//! overrides the positional count and highlights the parameter with that
//! label, since labelled struct-constructor arguments may appear in any order.

use std::path::Path;

use galvan_ast::{FnDecl, Span, ToplevelItem, TypeDecl, TypeElement};
use galvan_hir::query;
use tower_lsp::lsp_types::{
    Documentation, MarkupContent, MarkupKind, ParameterInformation, ParameterLabel, Position,
    SignatureHelp, SignatureInformation,
};

use crate::analysis::receiver_type_name;
use crate::document::Document;
use crate::features::{doc_comment, is_ident_byte, span_text};
use crate::workspace::Crate;

pub fn signature_help(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
) -> Option<SignatureHelp> {
    let offset = current.line_index.offset(&current.text, position)?;
    let call = call_at(&current.text, offset)?;
    let callee = callee_of(&current.text, call.open)?;

    // Typing the receiver requires the analysis; when it is unavailable (or
    // the file is broken and absent from it), fall back to offering every
    // method of the right name regardless of receiver.
    let receiver_type = match (&callee, file) {
        (Callee::Method { dot, .. }, Some(file)) => krate
            .analyze()
            .and_then(|analysis| {
                query::expression_at(&analysis.module, file, dot.saturating_sub(1))
            })
            .and_then(|receiver| receiver_type_name(&receiver.ty))
            .map(str::to_owned),
        _ => None,
    };

    let signatures = candidates(krate, &callee, receiver_type.as_deref());
    if signatures.is_empty() {
        return None;
    }

    let arg_label = current_arg_label(&current.text, call.arg_start, offset);
    let (active_signature, active_parameter) = select(&signatures, call.positional, arg_label);

    Some(SignatureHelp {
        signatures: signatures.into_iter().map(to_lsp).collect(),
        active_signature: Some(active_signature as u32),
        active_parameter,
    })
}

// ----------------------------------------------------------------------
// Call-site detection
// ----------------------------------------------------------------------

/// The innermost call argument list the cursor is inside.
struct CallSite {
    /// Byte offset of the call's `(`.
    open: usize,
    /// Number of commas at the call's own nesting level before the cursor,
    /// i.e. the zero-based index of the argument being typed.
    positional: usize,
    /// Byte offset just past the last of those commas (or past the `(`):
    /// where the current argument's text begins.
    arg_start: usize,
}

/// Scan the document from the start up to `offset`, tracking bracket nesting,
/// string literals (including `\( ... )` interpolations), char literals and
/// line comments, and return the innermost `(` group still open at the
/// cursor. Scanning forward (rather than backwards from the cursor) is what
/// makes string and comment states unambiguous.
fn call_at(text: &str, offset: usize) -> Option<CallSite> {
    enum Frame {
        /// A `(` group — a call candidate. Argument commas are counted here.
        Call {
            open: usize,
            commas: usize,
            arg_start: usize,
        },
        /// A `[` or `{` group: commas inside belong to it, not to the call.
        Group,
        /// Inside a `"…"` string literal.
        Str,
        /// Inside a `\( … )` string interpolation (code mode again).
        Interp,
    }

    let bytes = text.as_bytes();
    let end = offset.min(bytes.len());
    let mut stack: Vec<Frame> = Vec::new();

    /// Pop up to and including the innermost frame matching `wanted`,
    /// discarding frames left open above it (mid-edit error recovery).
    fn close(stack: &mut Vec<Frame>, wanted: impl Fn(&Frame) -> bool) {
        if let Some(at) = stack.iter().rposition(wanted) {
            stack.truncate(at);
        }
    }

    let mut i = 0;
    while i < end {
        if matches!(stack.last(), Some(Frame::Str)) {
            match bytes[i] {
                b'\\' if bytes.get(i + 1) == Some(&b'(') => {
                    stack.push(Frame::Interp);
                    i += 2;
                }
                b'\\' => i += 2, // escape sequence
                b'"' => {
                    stack.pop();
                    i += 1;
                }
                _ => i += 1,
            }
            continue;
        }
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < end && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'#' if bytes.get(i + 1) == Some(&b'"') => {
                // Raw string literal `#"…"#`: no escapes, no interpolation.
                i += 2;
                while i < end && !(bytes[i] == b'"' && bytes.get(i + 1) == Some(&b'#')) {
                    i += 1;
                }
                i = (i + 2).min(end);
            }
            b'"' => {
                stack.push(Frame::Str);
                i += 1;
            }
            b'\'' => {
                // Char literal: `'x'` or `'\n'`.
                i += 1;
                if bytes.get(i) == Some(&b'\\') {
                    i += 1;
                }
                while i < end && bytes[i] != b'\'' {
                    i += 1;
                }
                i += 1;
            }
            b'(' => {
                stack.push(Frame::Call {
                    open: i,
                    commas: 0,
                    arg_start: i + 1,
                });
                i += 1;
            }
            b'[' | b'{' => {
                stack.push(Frame::Group);
                i += 1;
            }
            b')' => {
                close(&mut stack, |frame| {
                    matches!(frame, Frame::Call { .. } | Frame::Interp)
                });
                i += 1;
            }
            b']' | b'}' => {
                close(&mut stack, |frame| matches!(frame, Frame::Group));
                i += 1;
            }
            b',' => {
                if let Some(Frame::Call { commas, arg_start, .. }) = stack.last_mut() {
                    *commas += 1;
                    *arg_start = i + 1;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }

    stack.iter().rev().find_map(|frame| match frame {
        Frame::Call {
            open,
            commas,
            arg_start,
        } => Some(CallSite {
            open: *open,
            positional: *commas,
            arg_start: *arg_start,
        }),
        _ => None,
    })
}

/// What is being called, read from the tokens before the `(`.
enum Callee {
    /// `name(` — a free function.
    Free { name: String },
    /// `expr.name(` or `expr?.name(` — a method; `dot` is the offset of the
    /// `.`, so the receiver expression ends just before it.
    Method { name: String, dot: usize },
    /// `Type(` — a struct or tuple constructor.
    Constructor { name: String },
    /// `Enum::Case(` — an enum case constructor.
    EnumCase { owner: String, case: String },
}

fn callee_of(text: &str, open: usize) -> Option<Callee> {
    let bytes = text.as_bytes();

    let mut end = open;
    while end > 0 && matches!(bytes[end - 1], b' ' | b'\t') {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && is_ident_byte(bytes[start - 1]) {
        start -= 1;
    }
    if start == end || !bytes[start].is_ascii_alphabetic() {
        return None; // A grouping paren, not a call.
    }
    let name = &text[start..end];
    let uppercase = bytes[start].is_ascii_uppercase();

    if start >= 2 && bytes[start - 1] == b':' && bytes[start - 2] == b':' {
        let qualifier_end = start - 2;
        let mut qualifier_start = qualifier_end;
        while qualifier_start > 0 && is_ident_byte(bytes[qualifier_start - 1]) {
            qualifier_start -= 1;
        }
        let qualifier = &text[qualifier_start..qualifier_end];
        if uppercase && qualifier.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
            return Some(Callee::EnumCase {
                owner: qualifier.to_string(),
                case: name.to_string(),
            });
        }
        // `pkg::function(` — external packages are not indexed (yet).
        return None;
    }
    if uppercase {
        return Some(Callee::Constructor {
            name: name.to_string(),
        });
    }
    if start > 0 && bytes[start - 1] == b'.' {
        return Some(Callee::Method {
            name: name.to_string(),
            dot: start - 1,
        });
    }
    Some(Callee::Free {
        name: name.to_string(),
    })
}

/// The label of the argument currently being typed (`label: value`), if it
/// has one. `arg_start` is where the argument's text begins.
fn current_arg_label(text: &str, arg_start: usize, offset: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let end = offset.min(bytes.len());

    let mut start = arg_start;
    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    let mut label_end = start;
    while label_end < end && is_ident_byte(bytes[label_end]) {
        label_end += 1;
    }
    if label_end == start || !bytes[start].is_ascii_lowercase() {
        return None;
    }
    let mut colon = label_end;
    while colon < end && matches!(bytes[colon], b' ' | b'\t') {
        colon += 1;
    }
    if bytes.get(colon) == Some(&b':') && bytes.get(colon + 1) != Some(&b':') {
        Some(&text[start..label_end])
    } else {
        None
    }
}

// ----------------------------------------------------------------------
// Candidate signatures
// ----------------------------------------------------------------------

/// A signature ready to convert to LSP: a rendered label and the byte ranges
/// of the call-site parameters within it.
struct SignatureData {
    label: String,
    params: Vec<ParamData>,
    documentation: Option<String>,
}

struct ParamData {
    /// Byte range of this parameter's text inside the label.
    range: (usize, usize),
    /// The name a labelled argument must match: the parameter's external
    /// label (or its name), a struct field's name, an enum field's name.
    name: Option<String>,
}

/// All declarations across the crate that `callee` may refer to. Every file
/// that parses contributes, so signatures keep working while the *current*
/// file is broken mid-edit.
fn candidates(krate: &Crate, callee: &Callee, receiver_type: Option<&str>) -> Vec<SignatureData> {
    let mut signatures = Vec::new();
    for file in krate.files() {
        let Some(segmented) = file.segmented.as_ref() else {
            continue;
        };
        let text = file.source.content();

        match callee {
            Callee::Free { name } => {
                for func in &segmented.functions {
                    if func.signature.identifier.as_str() == name
                        && func.signature.receiver().is_none()
                    {
                        signatures.push(function_signature(func, text));
                    }
                }
            }
            Callee::Method { name, .. } => {
                for func in &segmented.functions {
                    let Some(receiver) = func.signature.receiver() else {
                        continue;
                    };
                    if func.signature.identifier.as_str() != name {
                        continue;
                    }
                    if let (Some(expected), Some(actual)) =
                        (receiver_type, receiver_type_name(&receiver.param_type))
                    {
                        if expected != actual {
                            continue;
                        }
                    }
                    signatures.push(function_signature(func, text));
                }
            }
            Callee::Constructor { name } => {
                for decl in &segmented.types {
                    if decl.ident().as_str() == name {
                        signatures.extend(constructor_signature(decl, text));
                    }
                }
            }
            Callee::EnumCase { owner, case } => {
                for decl in &segmented.types {
                    let TypeDecl::Enum(decl_enum) = &decl.item else {
                        continue;
                    };
                    if decl_enum.ident.as_str() != owner {
                        continue;
                    }
                    for member in &decl_enum.members {
                        if member.ident.as_str() == case {
                            signatures.push(enum_case_signature(
                                owner,
                                &decl_enum.common_fields,
                                member,
                                text,
                            ));
                        }
                    }
                }
            }
        }
    }
    signatures
}

/// Render a function declaration as `fn name(params) -> Return`, taking each
/// parameter's text from the source. A `self` receiver is shown but is not a
/// call-site parameter (it is not counted by the active-argument index).
fn function_signature(func: &ToplevelItem<FnDecl>, source_text: &str) -> SignatureData {
    let signature = &func.signature;
    let mut label = format!("fn {}(", signature.identifier.as_str());
    let mut params = Vec::new();

    for (i, param) in signature.parameters.params.iter().enumerate() {
        if i > 0 {
            label.push_str(", ");
        }
        let start = label.len();
        label.push_str(&normalize_whitespace(span_text(source_text, param.span)));
        if param.identifier.is_self() {
            continue;
        }
        let name = param.call_label().unwrap_or(&param.identifier);
        params.push(ParamData {
            range: (start, label.len()),
            name: Some(name.as_str().to_string()),
        });
    }
    label.push(')');
    match &signature.return_type {
        TypeElement::Void(_) | TypeElement::Infer(_) => {}
        ty => {
            label.push_str(" -> ");
            label.push_str(&ty.to_string());
        }
    }

    SignatureData {
        label,
        params,
        documentation: doc_comment(source_text, signature.span),
    }
}

/// Render a struct constructor as `Type(field: Type, …)` or a tuple
/// constructor as `Type(Type, …)`. Alias, enum and empty types have no
/// constructor call form.
fn constructor_signature(decl: &ToplevelItem<TypeDecl>, source_text: &str) -> Option<SignatureData> {
    let (name, members): (_, Vec<(Option<String>, Span)>) = match &decl.item {
        TypeDecl::Struct(decl_struct) => (
            decl_struct.ident.as_str(),
            decl_struct
                .members
                .iter()
                .map(|member| (Some(member.ident.as_str().to_string()), member.span))
                .collect(),
        ),
        TypeDecl::Tuple(decl_tuple) => (
            decl_tuple.ident.as_str(),
            decl_tuple
                .members
                .iter()
                .map(|member| (None, member.span))
                .collect(),
        ),
        TypeDecl::Enum(_) | TypeDecl::Alias(_) | TypeDecl::Empty(_) => return None,
    };

    let mut label = format!("{name}(");
    let mut params = Vec::new();
    for (i, (member_name, span)) in members.iter().enumerate() {
        if i > 0 {
            label.push_str(", ");
        }
        let start = label.len();
        label.push_str(&normalize_whitespace(span_text(source_text, *span)));
        params.push(ParamData {
            range: (start, label.len()),
            name: member_name.clone(),
        });
    }
    label.push(')');

    Some(SignatureData {
        label,
        params,
        documentation: doc_comment(source_text, decl.item.span()),
    })
}

/// Render an enum case constructor as `Enum::Case(field: Type, …)`.
fn enum_case_signature(
    owner: &str,
    common_fields: &[galvan_ast::StructTypeMember],
    member: &galvan_ast::EnumTypeMember,
    source_text: &str,
) -> SignatureData {
    let mut label = format!("{owner}::{}(", member.ident.as_str());
    let mut params = Vec::new();
    for (i, field) in common_fields.iter().enumerate() {
        if i > 0 {
            label.push_str(", ");
        }
        let start = label.len();
        label.push_str(&normalize_whitespace(span_text(source_text, field.span)));
        params.push(ParamData {
            range: (start, label.len()),
            name: Some(field.ident.as_str().to_string()),
        });
    }
    for (i, field) in member.fields.iter().enumerate() {
        if i > 0 || !common_fields.is_empty() {
            label.push_str(", ");
        }
        let start = label.len();
        label.push_str(&normalize_whitespace(span_text(source_text, field.span)));
        params.push(ParamData {
            range: (start, label.len()),
            name: field.name.as_ref().map(|name| name.as_str().to_string()),
        });
    }
    label.push(')');

    SignatureData {
        label,
        params,
        documentation: doc_comment(source_text, member.span),
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ----------------------------------------------------------------------
// Active signature / parameter selection
// ----------------------------------------------------------------------

/// Choose which signature to present and which of its parameters to
/// highlight. A labelled argument selects by name (labelled constructor
/// arguments may be given in any order); otherwise the positional index
/// selects the first overload with enough parameters. `None` for the
/// parameter when the argument index is beyond every overload.
fn select(
    signatures: &[SignatureData],
    positional: usize,
    arg_label: Option<&str>,
) -> (usize, Option<u32>) {
    if let Some(label) = arg_label {
        for (i, signature) in signatures.iter().enumerate() {
            if let Some(j) = signature
                .params
                .iter()
                .position(|param| param.name.as_deref() == Some(label))
            {
                return (i, Some(j as u32));
            }
        }
    }
    if let Some(i) = signatures
        .iter()
        .position(|signature| signature.params.len() > positional)
    {
        return (i, Some(positional as u32));
    }
    (0, None)
}

fn to_lsp(signature: SignatureData) -> SignatureInformation {
    let parameters = signature
        .params
        .iter()
        .map(|param| ParameterInformation {
            label: ParameterLabel::LabelOffsets([
                utf16_offset(&signature.label, param.range.0),
                utf16_offset(&signature.label, param.range.1),
            ]),
            documentation: None,
        })
        .collect();
    SignatureInformation {
        label: signature.label,
        documentation: signature.documentation.map(|doc| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc,
            })
        }),
        parameters: Some(parameters),
        active_parameter: None,
    }
}

/// LSP parameter-label offsets are UTF-16 code units into the label string.
fn utf16_offset(label: &str, byte: usize) -> u32 {
    label[..byte].chars().map(char::len_utf16).sum::<usize>() as u32
}
