use galvan_ast::{Expression, Span, UnaryExpression, UnaryOperator};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for UnaryExpression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "unary_expression");
        let span = Span::from_node(node);

        cursor.child();
        cursor_expect!(cursor, "not");
        cursor.next();
        let operand = Box::new(Expression::read_cursor(cursor, source)?);
        cursor.goto_parent();

        Ok(Self {
            operator: UnaryOperator::LogicalNot,
            operand,
            span,
        })
    }
}
