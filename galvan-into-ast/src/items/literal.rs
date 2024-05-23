use galvan_ast::{Span, StringLiteral};

use crate::{cursor_expect, ReadCursor, SpanExt};

impl ReadCursor for StringLiteral {
    fn read_cursor(cursor: &mut galvan_parse::TreeCursor<'_>, source: &str) -> Result<Self, crate::AstError> {
        let node = cursor_expect!(cursor, "string_literal");
        let span = Span::from_node(node);

        let value = source[node.start_byte()..=node.end_byte()].to_owned();

        Ok(Self { value, span })
    }
}
