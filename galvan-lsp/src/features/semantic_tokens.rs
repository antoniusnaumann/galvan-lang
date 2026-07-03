//! `textDocument/semanticTokens/full`.
//!
//! Tokens come from two layers:
//!
//! 1. **Syntax** (the document's tree-sitter parse tree): comments, string /
//!    char / number literals and the grammar's keyword tokens. String
//!    interpolations (`"\(expr)"`) are excluded from the string token so the
//!    embedded expression is highlighted as code.
//! 2. **Semantics** (the typechecker's symbol index): every `ident` /
//!    `type_ident` resolves to its definition and is classified as function,
//!    method, struct, enum, enum member, property, parameter or variable —
//!    with a `declaration` modifier on the defining occurrence itself.
//!
//! Identifiers the index cannot resolve fall back to syntax-level classes:
//! the contextual control words (`if`, `for`, `while`, … — grammar-wise
//! ordinary identifiers) as keywords, the builtin statement functions
//! (`println`, …) and builtin types (`Int`, `String`, …) with a
//! `defaultLibrary` modifier. Everything else is left uncolored for the
//! client's syntactic highlighting.

use std::path::Path;

use galvan_parse::Node;
use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
    SemanticTokensLegend,
};

use crate::analysis::enum_type_names;
use crate::document::Document;
use crate::features::completion::BUILTIN_FUNCTIONS;
use crate::position::LineIndex;
use crate::workspace::{Analysis, Crate};

/// The token classes this server emits. [`legend`] must list the
/// [`SemanticTokenType`]s in exactly this order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TokenKind {
    Keyword,
    Comment,
    Str,
    Number,
    Function,
    Method,
    Struct,
    Enum,
    EnumMember,
    Property,
    Parameter,
    Variable,
}

/// Bit positions of the modifiers in [`legend`].
const MODIFIER_DECLARATION: u32 = 1 << 0;
const MODIFIER_DEFAULT_LIBRARY: u32 = 1 << 1;

/// The legend advertised in the server capabilities. Token-type order matches
/// [`TokenKind`]; modifier order matches the `MODIFIER_*` bit positions.
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::STRUCT,
            SemanticTokenType::ENUM,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::VARIABLE,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ],
    }
}

/// Contextual identifiers that read as control-flow keywords. They are not
/// grammar keywords (they parse as trailing-closure heads / free functions),
/// so they only take the keyword color when the index does not resolve them
/// to a user-defined symbol of the same name.
const CONTEXTUAL_KEYWORDS: &[&str] = &["if", "for", "while", "loop", "try", "return", "throw"];

pub fn semantic_tokens(current: &Document, krate: &Crate, file: Option<&Path>) -> SemanticTokens {
    let mut raw = Vec::new();
    if let Some(tree) = current.tree.as_ref() {
        let analysis = krate.analyze();
        let enums = analysis
            .map(|analysis| enum_type_names(&analysis.index))
            .unwrap_or_default();
        let mut collector = Collector {
            text: &current.text,
            analysis,
            enums,
            file,
            raw: &mut raw,
        };
        collector.walk(tree.root_node());
    }

    raw.sort_by_key(|token| token.start);
    SemanticTokens {
        result_id: None,
        data: encode(&current.text, &current.line_index, &raw),
    }
}

struct RawToken {
    start: usize,
    end: usize,
    kind: TokenKind,
    modifiers: u32,
}

struct Collector<'a> {
    text: &'a str,
    analysis: Option<&'a Analysis>,
    /// Names of enum types (precomputed; the index only knows `Type`).
    enums: std::collections::HashSet<&'a str>,
    file: Option<&'a Path>,
    raw: &'a mut Vec<RawToken>,
}

impl Collector<'_> {
    fn walk(&mut self, node: Node<'_>) {
        match node.kind() {
            "comment" => self.push(&node, TokenKind::Comment, 0),
            "string_literal" => self.string_literal(node),
            "raw_string_literal" | "char_literal" => self.push(&node, TokenKind::Str, 0),
            "number_literal" => self.push(&node, TokenKind::Number, 0),
            // Word spellings of operator tokens (`and`, `or`, `not`, `xor`,
            // `in`) read as keywords; their symbol spellings stay uncolored.
            "and" | "or" | "xor" | "not" | "contains"
                if self
                    .node_text(&node)
                    .bytes()
                    .all(|byte| byte.is_ascii_alphabetic()) =>
            {
                self.push(&node, TokenKind::Keyword, 0)
            }
            "ident" => self.ident(node),
            "type_ident" => self.type_ident(node),
            kind if kind.ends_with("_keyword") => self.push(&node, TokenKind::Keyword, 0),
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.walk(child);
                }
            }
        }
    }

    /// A string literal is one string token minus its interpolations, whose
    /// expressions are walked as ordinary code.
    fn string_literal(&mut self, node: Node<'_>) {
        let mut segment_start = node.start_byte();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "string_interpolation" {
                self.push_range(segment_start, child.start_byte(), TokenKind::Str, 0);
                self.walk(child);
                segment_start = child.end_byte();
            }
        }
        self.push_range(segment_start, node.end_byte(), TokenKind::Str, 0);
    }

    fn ident(&mut self, node: Node<'_>) {
        if let Some((kind, modifiers)) = self.resolve(&node) {
            self.push(&node, kind, modifiers);
            return;
        }
        let name = self.node_text(&node);
        if CONTEXTUAL_KEYWORDS.contains(&name) {
            self.push(&node, TokenKind::Keyword, 0);
        } else if BUILTIN_FUNCTIONS.contains(&name) {
            self.push(&node, TokenKind::Function, MODIFIER_DEFAULT_LIBRARY);
        }
    }

    fn type_ident(&mut self, node: Node<'_>) {
        if let Some((kind, modifiers)) = self.resolve(&node) {
            self.push(&node, kind, modifiers);
            return;
        }
        // Unresolved type identifiers are still types syntactically; builtins
        // (`Int`, `String`, …) additionally get the library modifier.
        let name = self.node_text(&node);
        let builtin = galvan_hir::builtins::builtins()
            .types
            .keys()
            .any(|ident| ident.as_str() == name);
        let modifiers = if builtin { MODIFIER_DEFAULT_LIBRARY } else { 0 };
        self.push(&node, TokenKind::Struct, modifiers);
    }

    /// Classify an identifier through the symbol index: the definition it is
    /// (with the `declaration` modifier) or the one it references.
    fn resolve(&self, node: &Node<'_>) -> Option<(TokenKind, u32)> {
        let (analysis, file) = (self.analysis?, self.file?);
        let offset = node.start_byte();

        let (id, modifiers) = match analysis.index.definition_at(file, offset) {
            Some(id) => (id, MODIFIER_DECLARATION),
            None => (analysis.index.reference_at(file, offset)?, 0),
        };
        let definition = analysis.index.definition(id);

        use galvan_hir::DefinitionKind;
        let kind = match &definition.kind {
            DefinitionKind::Local { .. } => TokenKind::Variable,
            DefinitionKind::Parameter { .. } => TokenKind::Parameter,
            DefinitionKind::Function { receiver: None } => TokenKind::Function,
            DefinitionKind::Function { receiver: Some(_) } => TokenKind::Method,
            DefinitionKind::Type => {
                if self.enums.contains(definition.name.as_str()) {
                    TokenKind::Enum
                } else {
                    TokenKind::Struct
                }
            }
            DefinitionKind::Field { .. } => TokenKind::Property,
            DefinitionKind::EnumVariant { .. } => TokenKind::EnumMember,
        };
        Some((kind, modifiers))
    }

    fn node_text(&self, node: &Node<'_>) -> &str {
        self.text.get(node.start_byte()..node.end_byte()).unwrap_or("")
    }

    fn push(&mut self, node: &Node<'_>, kind: TokenKind, modifiers: u32) {
        self.push_range(node.start_byte(), node.end_byte(), kind, modifiers);
    }

    fn push_range(&mut self, start: usize, end: usize, kind: TokenKind, modifiers: u32) {
        if start < end {
            self.raw.push(RawToken {
                start,
                end,
                kind,
                modifiers,
            });
        }
    }
}

/// Delta-encode position-sorted tokens as the LSP wire format (line and
/// UTF-16 column deltas). Tokens spanning multiple lines (raw strings,
/// multi-line string literals) are split per line, since clients do not
/// generally support multi-line semantic tokens.
fn encode(text: &str, index: &LineIndex, raw: &[RawToken]) -> Vec<SemanticToken> {
    let mut data = Vec::new();
    let mut previous_line = 0u32;
    let mut previous_start = 0u32;

    for token in raw {
        let mut segment_start = token.start;
        while segment_start < token.end {
            let segment_end = text[segment_start..token.end]
                .find('\n')
                .map(|newline| segment_start + newline)
                .unwrap_or(token.end);

            if segment_start < segment_end {
                let start = index.position(text, segment_start);
                let end = index.position(text, segment_end);
                let delta_line = start.line - previous_line;
                let delta_start = if delta_line == 0 {
                    start.character - previous_start
                } else {
                    start.character
                };
                data.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: end.character - start.character,
                    token_type: token.kind as u32,
                    token_modifiers_bitset: token.modifiers,
                });
                previous_line = start.line;
                previous_start = start.character;
            }
            segment_start = segment_end + 1;
        }
    }
    data
}
