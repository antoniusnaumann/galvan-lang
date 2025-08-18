use galvan_ast::{
    BooleanLiteral, CharLiteral, Expression, ExpressionKind, Ident, Literal, NoneLiteral, NumberLiteral, Span, StringLiteral
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

// Function to parse an arbitrary expression from string content
fn parse_interpolation_expression(expr_content: &str, span: &Span) -> Result<Expression, AstError> {
    // Create a minimal wrapper to parse the expression
    // We'll wrap it in a simple statement that we can parse
    let wrapper_source = format!("fn __temp() {{ let __x = {}; }}", expr_content);
    let source = galvan_files::Source::Str(wrapper_source.clone().into());
    
    // Parse the wrapper source
    match galvan_parse::parse_source(&source) {
        Ok(parsed_tree) => {
            // Create a cursor from the parsed tree
            let mut cursor = parsed_tree.root_node().walk();
            
            // Navigate to find the expression we want
            // Structure: source -> function -> body -> statement -> declaration -> expression
            if cursor.child() && // Enter source
               cursor.child() && // Enter function
               cursor_goto_named(&mut cursor, "body") &&
               cursor.child() && // Enter body
               cursor.child() && // Enter statement
               cursor_goto_named(&mut cursor, "declaration") &&
               cursor.child() && // Enter declaration
               cursor_goto_named(&mut cursor, "expression") {
                
                // Found the expression, parse it
                match Expression::read_cursor(&mut cursor, &wrapper_source) {
                    Ok(expr) => Ok(expr),
                    Err(_) => {
                        // Fallback to creating identifier
                        create_fallback_expression(expr_content, span)
                    }
                }
            } else {
                // Navigation failed, create fallback
                create_fallback_expression(expr_content, span)
            }
        }
        Err(_) => {
            // Parsing failed, create fallback
            create_fallback_expression(expr_content, span)
        }
    }
}

// Helper function to navigate to a named child node
fn cursor_goto_named(cursor: &mut TreeCursor<'_>, name: &str) -> bool {
    loop {
        if cursor.kind().unwrap_or("") == name {
            return true;
        }
        if !cursor.goto_next_sibling() {
            return false;
        }
    }
}

// Helper function to create fallback expressions
fn create_fallback_expression(expr_content: &str, span: &Span) -> Result<Expression, AstError> {
    if expr_content.chars().all(|c| c.is_alphanumeric() || c == '_') {
        // Simple identifier
        Ok(Expression {
            kind: ExpressionKind::Ident(Ident::new(expr_content)),
            span: span.clone(),
        })
    } else {
        // Treat as identifier anyway - let the transpiler handle complex syntax
        Ok(Expression {
            kind: ExpressionKind::Ident(Ident::new(expr_content)),
            span: span.clone(),
        })
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
        let full_text = source[node.start_byte()..node.end_byte()].to_owned();

        let mut interpolations = Vec::new();
        
        // Simple approach: manually parse the string and replace interpolations with placeholders
        if full_text.contains('{') && full_text.contains('}') {
            let mut template = String::new();
            let mut chars = full_text.chars().peekable();
            let mut placeholder_index = 0;
            
            // Skip opening quote
            if chars.peek() == Some(&'"') {
                chars.next(); 
                template.push('"');
            }
            
            while let Some(ch) = chars.next() {
                if ch == '{' {
                    // Found interpolation - find the closing brace
                    let mut expr_content = String::new();
                    let mut brace_depth = 1;
                    
                    while let Some(inner_ch) = chars.next() {
                        if inner_ch == '{' {
                            brace_depth += 1;
                        } else if inner_ch == '}' {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                break;
                            }
                        }
                        expr_content.push(inner_ch);
                    }
                    
                    // Parse arbitrary expressions within interpolations
                    let expr = parse_interpolation_expression(&expr_content, &span)?;
                    interpolations.push(expr);
                    
                    // Add placeholder to template
                    template.push_str(&format!("{{{}}}", placeholder_index));
                    placeholder_index += 1;
                } else {
                    template.push(ch);
                }
            }
            
            Ok(Self {
                value: template,
                interpolations,
                span,
            })
        } else {
            // No interpolation - return as-is
            Ok(Self {
                value: full_text,
                interpolations,
                span,
            })
        }
    }
}

impl ReadCursor for CharLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "char_literal");
        let span = Span::from_node(node);

        let text = &source[node.start_byte()..node.end_byte()];
        
        // Remove quotes and parse character
        let char_content = &text[1..text.len()-1]; // Remove surrounding quotes
        
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
                    let hex_str = &char_content[3..char_content.len()-1];
                    let code_point = u32::from_str_radix(hex_str, 16)
                        .map_err(|_| AstError::InvalidCharacterLiteral(span))?;
                    char::from_u32(code_point)
                        .ok_or(AstError::InvalidCharacterLiteral(span))?
                }
                _ => return Err(AstError::InvalidCharacterLiteral(span)),
            }
        } else {
            // Regular character
            char_content.chars().next()
                .ok_or(AstError::InvalidCharacterLiteral(span))?
        };

        Ok(CharLiteral { value, span })
    }
}


