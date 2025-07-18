use galvan_ast::{
    AccessExpression, Block, Body, ClosureParameter, ElseExpression, Expression, PostfixExpression,
    Span, YeetExpression,
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
        cursor_expect!(cursor, "else_expression");

        cursor.child();
        let receiver = Expression::read_cursor(cursor, source)?.into();

        cursor.next();
        cursor_expect!(cursor, "else_keyword");

        cursor.next();
        let mut parameter = Vec::new();
        if cursor.kind()? == "pipe" {
            cursor_expect!(cursor, "pipe");
            cursor.next();
            while cursor.kind()? == "closure_argument" {
                parameter.push(ClosureParameter::read_cursor(cursor, source)?);
                cursor.next();
                while cursor.kind()? == "," {
                    cursor.next();
                }
            }

            cursor_expect!(cursor, "pipe");
        }

        cursor.next();
        let body = Body::read_cursor(cursor, source)?;
        let body_span = body.span;
        let block = Block {
            body,
            span: body_span,
        };

        cursor.goto_parent();
        Ok(ElseExpression {
            receiver,
            block,
            parameters: parameter,
        })
    }
}
