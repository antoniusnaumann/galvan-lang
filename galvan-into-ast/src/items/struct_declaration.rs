use galvan_ast::{Span, StructTypeDecl, StructTypeMember, TypeElement, Visibility};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, read_visibility, AstError, ReadCursor, SpanExt};

impl ReadCursor for StructTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let struct_decl = cursor_expect!(cursor, "struct");
        let span = Span::from_node(struct_decl);

        cursor.goto_first_child();
        
        let visibility = read_visibility(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.goto_next_sibling();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.goto_next_sibling();
        cursor_expect!(cursor, "brace_open");

        cursor.goto_next_sibling();
        let mut members = vec![];
        while cursor.kind()? == "struct_field" {
            let field = StructTypeMember::read_cursor(cursor, source)?;
            members.append(field);

            cursor.goto_next_sibling();
        }

        cursor_expect!(cursor, "brace_close");
        
        cursor.goto_parent();
        
        Ok(StructTypeDecl { visibility, ident, members, span })
    }
}

impl ReadCursor for StructTypeMember {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let struct_field = cursor_expect!(cursor, "struct_field");
        let span = Span::from_node(struct_field);

        cursor.goto_first_child();

        let visibility = read_visibility(cursor)?;
        let decl_modifier = read_decl_modifier(cursor)?;

        cursor.goto_next_sibling();
        let ident = Ident::read_cursor(cursor, source)?;

        cursor.goto_next_sibling();
        cursor_expect!(cursor, "colon");

        cursor.goto_next_sibling();
        let r#type = TypeElement::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(StructTypeMember { decl_modifier, ident, r#type, span })
    }
}
