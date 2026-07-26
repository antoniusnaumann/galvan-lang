//! A Wadler-style pretty-printing document.
//!
//! The emitter (`emit.rs`) lowers the parse tree into this IR; the renderer
//! here decides line breaks: a [`Doc::Group`] is printed on one line when it
//! fits within the configured width and is otherwise *broken*, turning every
//! [`Doc::Line`]/[`Doc::SoftLine`] inside it (but not inside nested groups)
//! into a newline at the group's indentation.

use crate::FormatOptions;

#[derive(Debug, Clone)]
pub enum Doc {
    /// Verbatim text without newlines.
    Text(String),
    /// Verbatim text that may span lines (strings, comments). Anything after
    /// the first newline is emitted untouched: multi-line tokens own their
    /// internal layout. A group containing one of these never fits.
    RawText(String),
    /// Newline when broken, a single space when flat.
    Line,
    /// Newline when broken, nothing when flat.
    SoftLine,
    /// Always a newline.
    HardLine,
    /// Always an empty line (two newlines). Used to preserve the author's
    /// paragraph breaks.
    BlankLine,
    /// Emitted only when the enclosing group is broken (e.g. trailing commas).
    IfBreak(String),
    /// Fits-on-one-line decision point.
    Group(Vec<Doc>),
    /// Increase indentation by one unit for the contents.
    Indent(Vec<Doc>),
    /// A sequence with no layout meaning of its own.
    Concat(Vec<Doc>),
}

impl Doc {
    pub fn text(text: impl Into<String>) -> Self {
        Doc::Text(text.into())
    }

    /// Width when printed flat, or `None` if it can never be flat.
    fn flat_width(&self) -> Option<usize> {
        match self {
            Doc::Text(text) => Some(text.chars().count()),
            Doc::RawText(text) => {
                if text.contains('\n') {
                    None
                } else {
                    Some(text.chars().count())
                }
            }
            Doc::Line => Some(1),
            Doc::SoftLine => Some(0),
            Doc::HardLine | Doc::BlankLine => None,
            Doc::IfBreak(_) => Some(0),
            Doc::Group(children) | Doc::Indent(children) | Doc::Concat(children) => {
                flat_width(children)
            }
        }
    }

    pub fn render(&self, options: &FormatOptions) -> String {
        let mut out = String::new();
        let mut column = 0usize;
        render(self, false, 0, &mut column, &mut out, options);
        out
    }
}

fn flat_width(children: &[Doc]) -> Option<usize> {
    children
        .iter()
        .try_fold(0usize, |acc, child| Some(acc + child.flat_width()?))
}

fn indent_string(level: usize, options: &FormatOptions) -> String {
    if options.use_tabs {
        "\t".repeat(level)
    } else {
        " ".repeat(level * options.indent_width)
    }
}

/// The column an indent unit occupies for width accounting (a tab counts as
/// the indent width so tab and space layouts break identically).
fn indent_columns(level: usize, options: &FormatOptions) -> usize {
    level * options.indent_width
}

fn render(
    doc: &Doc,
    broken: bool,
    indent: usize,
    column: &mut usize,
    out: &mut String,
    options: &FormatOptions,
) {
    match doc {
        Doc::Text(text) => {
            out.push_str(text);
            *column += text.chars().count();
        }
        Doc::RawText(text) => {
            out.push_str(text);
            match text.rsplit_once('\n') {
                Some((_, last)) => *column = last.chars().count(),
                None => *column += text.chars().count(),
            }
        }
        Doc::Line => {
            if broken {
                newline(indent, column, out, options);
            } else {
                out.push(' ');
                *column += 1;
            }
        }
        Doc::SoftLine => {
            if broken {
                newline(indent, column, out, options);
            }
        }
        Doc::HardLine => newline(indent, column, out, options),
        Doc::BlankLine => {
            // Trailing whitespace on the empty line is never wanted.
            out.push('\n');
            newline(indent, column, out, options);
        }
        Doc::IfBreak(text) => {
            if broken {
                out.push_str(text);
                *column += text.chars().count();
            }
        }
        Doc::Group(children) => {
            let fits =
                flat_width(children).is_some_and(|width| *column + width <= options.max_width);
            for child in children {
                render(child, !fits, indent, column, out, options);
            }
        }
        Doc::Indent(children) => {
            for child in children {
                render(child, broken, indent + 1, column, out, options);
            }
        }
        Doc::Concat(children) => {
            for child in children {
                render(child, broken, indent, column, out, options);
            }
        }
    }
}

fn newline(indent: usize, column: &mut usize, out: &mut String, options: &FormatOptions) {
    // Never leave trailing whitespace before a line break.
    while out.ends_with(' ') || out.ends_with('\t') {
        out.pop();
    }
    out.push('\n');
    out.push_str(&indent_string(indent, options));
    *column = indent_columns(indent, options);
}
