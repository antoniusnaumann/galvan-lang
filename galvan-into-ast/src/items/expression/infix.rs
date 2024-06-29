use galvan_ast::{Group, InfixExpression};
use galvan_parse::TreeCursor;

use crate::{AstError, ReadCursor};
 
impl ReadCursor for Group {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for InfixExpression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

