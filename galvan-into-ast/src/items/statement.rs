use crate::result::CursorUtil;
use crate::{cursor_expect, AstError, ReadCursor};
use galvan_ast::{Assignment, Declaration, Expression, FunctionCall, Statement};
use galvan_parse::TreeCursor;

impl ReadCursor for Statement {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "statement");

        cursor.goto_first_child();
        let inner = match cursor.kind()? {
            "assignment" => Statement::Assignment(Assignment::read_cursor(cursor, source)?),
            "declaration" => Statement::Declaration(Declaration::read_cursor(cursor, source)?),
            "expression" => Statement::Expression(Expression::read_cursor(cursor, source)?),
            "free_function" => Statement::Expression(FunctionCall::read_cursor(cursor, source)?),
            _ => unreachable!("Unknown statement kind: {:?}", cursor.kind()?),
        };

        cursor.goto_parent();
        Ok(inner)
    }
}

impl ReadCursor for Declaration {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!("Implement Declaration::read_cursor")
    }
}

impl ReadCursor for Assignment {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!("Implement Assignment::read_cursor")
    }
}

impl ReadCursor for Expression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!("Implement Expression::read_cursor")
    }
}

impl ReadCursor for FunctionCall {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!("Implement FunctionCall::read_cursor")
    }
}
