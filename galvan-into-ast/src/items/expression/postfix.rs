use galvan_ast::{
    AccessExpression, Block, Body, ElseExpression, Expression, PostfixExpression, Span, YeetExpression
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for PostfixExpression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "postfix_expression");
        let span = Span::from_node(node);

        cursor.child();
        let inner = Expression::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "postfix_operator");

        cursor.child();
        let res = match cursor.kind()? {
            "yeet_operator" => YeetExpression { inner, span }.into(),
            "access_operator" => {
                cursor.child();
                cursor_expect!(cursor, "bracket_open");

                cursor.next();
                let index = Expression::read_cursor(cursor, source)?;

                cursor.next();
                cursor_expect!(cursor, "bracket_close");

                cursor.goto_parent();
                AccessExpression {
                    base: inner,
                    index,
                    span,
                }
                .into()
            }
            unknown => unreachable!("Unknown postfix operator: {unknown}"),
        };
        cursor.goto_parent();

        cursor.goto_parent();
        Ok(res)
    }
}

impl ReadCursor for ElseExpression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "else_expression");
        let span = Span::from_node(node);
        
        cursor.child();
        let receiver = Expression::read_cursor(cursor, source)?.into();

        cursor.next();
        cursor_expect!(cursor, "else_keyword");

        cursor.next();
        let body = Body::read_cursor(cursor, source)?;
        let body_span = body.span;
        let block = Block { body, span: body_span };

        cursor.goto_parent();
        Ok(ElseExpression { receiver, block, span })
    }
}
