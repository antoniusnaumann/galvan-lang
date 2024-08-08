use galvan_ast::{
    AliasTypeDecl, DeclModifier, EmptyTypeDecl, Ident, Span, StructTypeDecl, StructTypeMember, TupleTypeDecl, TupleTypeMember, TypeElement, TypeIdent, Visibility
};
use galvan_parse::TreeCursor;

use crate::result::CursorUtil;
use crate::{cursor_expect, AstError, ReadCursor, SpanExt};

impl ReadCursor for StructTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let struct_decl = cursor_expect!(cursor, "struct");
        let span = Span::from_node(struct_decl);

        cursor.child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.next();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "brace_open");

        cursor.next();
        let mut members = vec![];
        while cursor.kind()? == "struct_field" {
            let field = StructTypeMember::read_cursor(cursor, source)?;
            members.push(field);

            cursor.next();
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

        cursor.child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        let decl_modifier = if cursor.kind()? == "declaration_modifier" {
            let modifier = Some(DeclModifier::read_cursor(cursor, source)?);
            cursor.next();
            modifier
        } else {
            None
        };

        let ident = Ident::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "colon");

        cursor.next();
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
        let node = cursor_expect!(cursor, "tuple_struct");
        let span = Span::from_node(node);

        cursor.child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.next();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "paren_open");

        cursor.next();
        let mut members = vec![];
        while cursor.kind()? == "tuple_field" {
            let field = TupleTypeMember::read_cursor(cursor, source)?;
            members.push(field);

            cursor.next();
        }

        cursor_expect!(cursor, "paren_close");

        cursor.goto_parent();

        Ok(TupleTypeDecl {
            visibility,
            ident,
            members,
            span,
        })
    }
}

impl ReadCursor for TupleTypeMember {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "tuple_field");
        let span = Span::from_node(node);

        cursor.child();

        // TODO: Implement field visibility and declaration modifier
        let visibility = Visibility::read_cursor(cursor, source)?;
        let decl_modifier = if cursor.kind()? == "declaration_modifier" {
            let modifier = Some(DeclModifier::read_cursor(cursor, source)?);
            cursor.next();
            modifier
        } else {
            None
        };


        cursor.next();
        let r#type = TypeElement::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(TupleTypeMember {
            // decl_modifier,
            r#type,
            span,
        })
    }
}

impl ReadCursor for AliasTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let alias = cursor_expect!(cursor, "alias");
        let span = Span::from_node(alias);

        cursor.child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.next();

        let ident = TypeIdent::read_cursor(cursor, source)?;
        cursor.next();

        cursor_expect!(cursor, "assign");

        cursor.next();
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

        cursor.child();
        let visibility = Visibility::read_cursor(cursor, source)?;

        cursor.next();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(EmptyTypeDecl {
            visibility,
            ident,
            span,
        })
    }
}
