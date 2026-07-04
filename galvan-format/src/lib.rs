//! Source-code formatter for the Galvan programming language.
//!
//! The formatter parses with the real tree-sitter grammar and pretty-prints
//! the concrete syntax tree, so it normalizes token spacing, indentation and
//! line breaks while preserving comments, blank-line paragraphs and the exact
//! spelling of every token (including Unicode operator variants like `≠`).
//!
//! It deliberately refuses to format files with syntax errors: rewriting a
//! broken tree would move code around based on garbage structure.
//!
//! The style it implements is documented in `STYLE.md` next to this crate.

mod doc;
mod emit;

use galvan_files::Source;
use galvan_parse::parse_source;
use thiserror::Error;

/// Spelling of operators that exist in a Unicode and an ASCII variant
/// (`≠`/`!=`, `≥`/`>=`, `≤`/`<=`, `≡`/`===`, `≢`/`!==`, `→`/`->`,
/// `⇒`/`=>`, `±`/`+-`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnicodeStyle {
    /// Keep whichever spelling the author wrote.
    #[default]
    Untouched,
    /// Normalize to the Unicode spelling (`≠`, `→`, `±`).
    Unicode,
    /// Normalize to the ASCII spelling (`!=`, `->`, `+-`).
    Ascii,
}

/// Spelling of logical/containment operators that exist as a word and as a
/// symbol (`and`/`&&`, `or`/`||`, `not`/`!`, `in`/`∈`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogicalStyle {
    /// Keep whichever spelling the author wrote.
    #[default]
    Untouched,
    /// Normalize to the word spelling (`and`, `or`, `not`, `in`).
    Word,
    /// Normalize to the symbol spelling (`&&`, `||`, `!`, `∈`).
    Symbol,
}

/// Formatting configuration. [`FormatOptions::default`] is the canonical
/// Galvan style: 4-space indents, 100-column line width, operator
/// spellings left untouched.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Columns per indentation level (also the width a tab is accounted as).
    pub indent_width: usize,
    /// Indent with tabs instead of spaces.
    pub use_tabs: bool,
    /// Target maximum line width. Lines that cannot be broken (long tokens,
    /// deeply nested code) may still exceed it.
    pub max_width: usize,
    /// Normalize Unicode/ASCII operator pairs to one spelling.
    pub unicode_operators: UnicodeStyle,
    /// Normalize word/symbol logical-operator pairs to one spelling.
    pub logical_operators: LogicalStyle,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_width: 4,
            use_tabs: false,
            max_width: 100,
            unicode_operators: UnicodeStyle::default(),
            logical_operators: LogicalStyle::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("could not parse source")]
    Parse,
    #[error("source contains syntax errors; not formatting")]
    SyntaxErrors,
}

/// Format a whole Galvan source file. The result always ends in exactly one
/// trailing newline (empty input stays empty).
pub fn format_source(text: &str, options: &FormatOptions) -> Result<String, FormatError> {
    let tree = parse_source(&Source::from_string(text.to_owned())).map_err(|_| FormatError::Parse)?;
    if tree.root_node().has_error() {
        return Err(FormatError::SyntaxErrors);
    }

    let doc = emit::source_doc(tree.root_node(), text, options);
    let mut formatted = doc.render(options);

    while formatted.ends_with('\n') || formatted.ends_with(' ') || formatted.ends_with('\t') {
        formatted.pop();
    }
    if !formatted.is_empty() {
        formatted.push('\n');
    }
    Ok(formatted)
}
