use galvan_ast::{
    BooleanLiteral, CharLiteral, Expression, Literal, NoneLiteral, NumberLiteral, Span,
    StringLiteral,
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

fn push_escaped_format_text(output: &mut String, text: &str) {
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                output.push(ch);
                let Some(escaped) = chars.next() else {
                    continue;
                };
                output.push(escaped);

                if escaped == 'u' && chars.peek() == Some(&'{') {
                    for unicode_char in chars.by_ref() {
                        output.push(unicode_char);
                        if unicode_char == '}' {
                            break;
                        }
                    }
                }
            }
            '{' => output.push_str("{{"),
            '}' => output.push_str("}}"),
            _ => output.push(ch),
        }
    }
}

fn read_interpolation(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Expression, AstError> {
    cursor_expect!(cursor, "string_interpolation");
    cursor.child();

    loop {
        if cursor.kind()? == "expression" {
            return Expression::read_cursor(cursor, source);
        }
        if !cursor.next() {
            return Err(AstError::ConversionError);
        }
    }
}

impl ReadCursor for Literal {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "literal");
        let span = Span::from_node(node);

        cursor.child();
        let inner = match cursor.kind()? {
            "none_keyword" => Literal::NoneLiteral(NoneLiteral(span)),
            "boolean_literal" => BooleanLiteral::read_cursor(cursor, source)?.into(),
            "string_literal" => StringLiteral::read_cursor(cursor, source)?.into(),
            "char_literal" => CharLiteral::read_cursor(cursor, source)?.into(),
            "number_literal" => NumberLiteral::read_cursor(cursor, source)?.into(),
            unknown => unreachable!("Unknown literal type: {unknown}"),
        };
        cursor.goto_parent();

        Ok(inner)
    }
}

impl ReadCursor for BooleanLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "boolean_literal");
        let span = Span::from_node(node);

        cursor.child();
        let lit = match cursor.kind()? {
            "true_keyword" => BooleanLiteral { value: true, span },
            "false_keyword" => BooleanLiteral { value: false, span },
            _ => unreachable!(),
        };

        cursor.goto_parent();
        Ok(lit)
    }
}

impl ReadCursor for NumberLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "number_literal");
        let span = Span::from_node(node);

        let value = source[node.start_byte()..node.end_byte()].to_owned();

        Ok(NumberLiteral { value, span })
    }
}

impl ReadCursor for StringLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "string_literal");
        let span = Span::from_node(node);
        let mut value = String::new();
        let mut interpolations = Vec::new();
        let mut last_byte = node.start_byte();
        let mut child_cursor = node.walk();

        if child_cursor.goto_first_child() {
            loop {
                let child = child_cursor.node();
                if child.kind() == "string_interpolation" {
                    push_escaped_format_text(&mut value, &source[last_byte..child.start_byte()]);
                    let mut interpolation_cursor = child.walk();
                    interpolations.push(read_interpolation(&mut interpolation_cursor, source)?);
                    value.push_str("{}");
                    last_byte = child.end_byte();
                }

                if !child_cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        push_escaped_format_text(&mut value, &source[last_byte..node.end_byte()]);

        Ok(Self {
            value,
            interpolations,
            span,
        })
    }
}

impl ReadCursor for CharLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "char_literal");
        let span = Span::from_node(node);

        let text = &source[node.start_byte()..node.end_byte()];

        // Remove quotes and parse character
        let char_content = &text[1..text.len() - 1]; // Remove surrounding quotes

        let value = if char_content.starts_with('\\') {
            // Handle escape sequences
            match char_content {
                "\\n" => '\n',
                "\\r" => '\r',
                "\\t" => '\t',
                "\\\\" => '\\',
                "\\'" => '\'',
                "\\\"" => '"',
                _ if char_content.starts_with("\\u{") && char_content.ends_with('}') => {
                    // Unicode escape: \u{1F600}
                    let hex_str = &char_content[3..char_content.len() - 1];
                    let code_point = u32::from_str_radix(hex_str, 16)
                        .map_err(|_| AstError::InvalidCharacterLiteral(span))?;
                    char::from_u32(code_point).ok_or(AstError::InvalidCharacterLiteral(span))?
                }
                _ => return Err(AstError::InvalidCharacterLiteral(span)),
            }
        } else {
            // Regular character
            char_content
                .chars()
                .next()
                .ok_or(AstError::InvalidCharacterLiteral(span))?
        };

        Ok(CharLiteral { value, span })
    }
}
