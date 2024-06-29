use galvan_ast::{Closure, ConstructorCall, ConstructorCallArg, Expression, FunctionCall, Ident, Span, TypeIdent};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for FunctionCall {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

pub fn read_trailing_closure_call(
    cursor: &mut TreeCursor<'_>,
    source: &str,
) -> Result<FunctionCall, AstError> {
    todo!()
}

pub fn read_free_function_call(
    cursor: &mut TreeCursor<'_>,
    source: &str,
) -> Result<FunctionCall, AstError> {
    todo!()
}

impl ReadCursor for Closure {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for ConstructorCall {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "constructor_call");
        let span = Span::from_node(node);

        cursor.goto_first_child();
        let identifier = TypeIdent::read_cursor(cursor, source)?;
        
        cursor.goto_next_sibling();
        cursor_expect!(cursor, "paren_open");

        let mut arguments = vec![];
        cursor.goto_next_sibling();
        while cursor.kind()? != "paren_close" {
            let arg = ConstructorCallArg::read_cursor(cursor, source)?;
            arguments.push(arg);
            cursor.goto_next_sibling();
        }

        cursor.goto_parent();

        let constructed = ConstructorCall {
            identifier,
            arguments,
            span,
        };
        Ok(constructed)
    }
}

impl ReadCursor for ConstructorCallArg {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "constructor_call_arg");
        let span = Span::from_node(node);

        cursor.goto_first_child();
        let ident = Ident::read_cursor(cursor, source)?;

        cursor.goto_next_sibling();
        cursor_expect!(cursor, "colon");

        cursor.goto_next_sibling();
        let expression = Expression::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(ConstructorCallArg { ident, expression, span } )
    }
}
