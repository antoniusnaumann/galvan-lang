use galvan_ast::{
    BooleanLiteral, Literal, NoneLiteral, NumberLiteral, Span, StringLiteral
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for Literal {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "literal");
        let span = Span::from_node(node);

        cursor.goto_first_child();
        let inner = match cursor.kind()? {
            "none_keyword" => Literal::NoneLiteral(NoneLiteral(span)),
            "boolean_literal" => BooleanLiteral::read_cursor(cursor, source)?.into(),
            "string_literal" => StringLiteral::read_cursor(cursor, source)?.into(),
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

        cursor.goto_first_child();
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

        let value = source[node.start_byte()..node.end_byte()].to_owned();

        Ok(Self { value, span })
    }
}


