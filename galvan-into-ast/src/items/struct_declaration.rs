use galvan_ast::{
    AliasTypeDecl, DeclModifier, EmptyTypeDecl, Ident, Span, StructTypeDecl, StructTypeMember,
    TupleTypeDecl, TypeElement, TypeIdent, Visibility,
};
use galvan_parse::TreeCursor;

use crate::result::CursorUtil;
use crate::{cursor_expect, AstError, ReadCursor, SpanExt};

impl ReadCursor for StructTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let struct_decl = cursor_expect!(cursor, "struct");
        let span = Span::from_node(struct_decl);

        cursor.goto_first_child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.goto_next_sibling();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.goto_next_sibling();
        cursor_expect!(cursor, "brace_open");

        cursor.goto_next_sibling();
        let mut members = vec![];
        while cursor.kind()? == "struct_field" {
            let field = StructTypeMember::read_cursor(cursor, source)?;
            members.push(field);

            cursor.goto_next_sibling();
        }

        cursor_expect!(cursor, "brace_close");

        cursor.goto_parent();

        Ok(StructTypeDecl {
            visibility,
            ident,
            members,
            span,
        })
    }
}

impl ReadCursor for StructTypeMember {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let struct_field = cursor_expect!(cursor, "struct_field");
        let span = Span::from_node(struct_field);

        cursor.goto_first_child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        let decl_modifier = if cursor.kind()? == "declaration_modifier" {
            Some(DeclModifier::read_cursor(cursor, source)?)
        } else {
            None
        };

        cursor.goto_next_sibling();
        let ident = Ident::read_cursor(cursor, source)?;

        cursor.goto_next_sibling();
        cursor_expect!(cursor, "colon");

        cursor.goto_next_sibling();
        let r#type = TypeElement::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(StructTypeMember {
            decl_modifier,
            ident,
            r#type,
            span,
        })
    }
}

impl ReadCursor for TupleTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!("Implement ast conversion for tuple type")
    }
}

impl ReadCursor for AliasTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let alias = cursor_expect!(cursor, "alias");
        let span = Span::from_node(alias);

        cursor.goto_first_child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.goto_next_sibling();

        let ident = TypeIdent::read_cursor(cursor, source)?;
        cursor.goto_next_sibling();

        cursor_expect!(cursor, "assign");

        cursor.goto_next_sibling();
        let r#type = TypeElement::read_cursor(cursor, source)?;
        cursor.goto_parent();

        Ok(Self {
            visibility,
            ident,
            r#type,
            span,
        })
    }
}

impl ReadCursor for EmptyTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let empty_type = cursor_expect!(cursor, "empty_struct");
        let span = Span::from_node(empty_type);

        cursor.goto_first_child();
        let visibility = Visibility::read_cursor(cursor, source)?;

        cursor.goto_next_sibling();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(EmptyTypeDecl {
            visibility,
            ident,
            span,
        })
    }
}
