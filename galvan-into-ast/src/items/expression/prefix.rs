use galvan_ast::{Expression, RefExpression};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor};

impl ReadCursor for RefExpression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "ref_expression");

        cursor.child();
        cursor_expect!(cursor, "ref_keyword");

        cursor.next();
        let inner = Box::new(Expression::read_cursor(cursor, source)?);

        cursor.goto_parent();
        Ok(RefExpression { inner })
    }
}
