use galvan_ast::{ElseExpression, PostfixExpression};
use galvan_parse::TreeCursor;

use crate::{AstError, ReadCursor};

impl ReadCursor for PostfixExpression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for ElseExpression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}
