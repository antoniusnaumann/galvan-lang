use galvan_ast::{
    ArrayLiteral, CollectionLiteral, DictLiteral, DictLiteralElement, Expression, ExpressionKind,
    OrderedDictLiteral, SetLiteral, Span,
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for CollectionLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "collection_literal");

        cursor.child();
        let inner = match cursor.kind()? {
            "array_literal" => ArrayLiteral::read_cursor(cursor, source)?.into(),
            "set_literal" => SetLiteral::read_cursor(cursor, source)?.into(),
            "ordered_dict_literal" => OrderedDictLiteral::read_cursor(cursor, source)?.into(),
            "dict_literal" => DictLiteral::read_cursor(cursor, source)?.into(),
            lit => unreachable!("Unknown collection literal type: {lit}"),
        };
        cursor.goto_parent();

        Ok(inner)
    }
}

impl ReadCursor for ArrayLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "array_literal");
        let span = Span::from_node(node);

        cursor.child();
        cursor_expect!(cursor, "bracket_open");

        cursor.next();
        let mut elements = Vec::new();
        while cursor.kind()? == "expression" {
            elements.push(Expression::read_cursor(cursor, source)?);
            cursor.next();
            while cursor.kind()? == "," {
                cursor.next();
            }
        }

        cursor_expect!(cursor, "bracket_close");
        cursor.goto_parent();

        Ok(ArrayLiteral { elements, span })
    }
}

impl ReadCursor for SetLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "set_literal");
        let span = Span::from_node(node);

        cursor.child();
        cursor_expect!(cursor, "brace_open");

        cursor.next();
        let mut elements = Vec::new();
        while cursor.kind()? == "expression" {
            elements.push(Expression::read_cursor(cursor, source)?);
            cursor.next();
        }

        cursor_expect!(cursor, "brace_close");
        cursor.goto_parent();

        Ok(SetLiteral { elements, span })
    }
}

impl ReadCursor for OrderedDictLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "ordered_dict_literal");
        let span = Span::from_node(node);

        cursor.child();
        cursor_expect!(cursor, "bracket_open");

        cursor.next();
        let mut elements = Vec::new();
        while cursor.kind()? == "dict_element" {
            elements.push(DictLiteralElement::read_cursor(cursor, source)?);
            cursor.next();
        }

        cursor_expect!(cursor, "bracket_open");
        cursor.goto_parent();

        Ok(OrderedDictLiteral { elements, span })
    }
}

impl ReadCursor for DictLiteral {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "dict_literal");
        let span = Span::from_node(node);

        cursor.child();
        cursor_expect!(cursor, "brace_open");

        cursor.next();
        let mut elements = Vec::new();
        while cursor.kind()? == "dict_element" {
            elements.push(DictLiteralElement::read_cursor(cursor, source)?);
            cursor.next();
        }

        cursor_expect!(cursor, "brace_close");
        cursor.goto_parent();

        Ok(DictLiteral { elements, span })
    }
}

impl ReadCursor for DictLiteralElement {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "dict_element");
        let span = Span::from_node(node);

        cursor.child();
        let key = Expression::read_cursor(cursor, source)?;

        cursor.next();
        let value = Expression::read_cursor(cursor, source)?;

        Ok(DictLiteralElement { key, value, span })
    }
}
