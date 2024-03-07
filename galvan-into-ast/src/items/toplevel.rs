use galvan_ast::{Body, FnDecl, FnSignature, Ident, MainDecl, ParamList, RootItem, Span, StringLiteral, TestDecl, TypeDecl, TypeElement, Visibility};
use galvan_parse::{Range, TreeCursor};

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for RootItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>) -> Result<Self, AstError> {
        Ok(match cursor.kind()? {
            "main" => MainDecl::read_cursor(cursor)?.into(),
            "build" => todo!("Implement build entry point!"),
            "test" => TestDecl::read_cursor(cursor)?.into(),
            "function" => FnDecl::read_cursor(cursor)?.into(),
            "type_declaration" => TypeDecl::read_cursor(cursor)?.into(),
            "entry_point" => todo!("Implement custom tasks!"),
            other => unreachable!("Unexpected node in root item: {other}"),
        })
    }
}

impl ReadCursor for MainDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>) -> Result<Self, AstError> {
        let main = cursor_expect!(cursor, "main");
        let span = Span::from_node(main);
        cursor.goto_first_child();
        cursor_expect!(cursor, "main_keyword");

        cursor.goto_next_sibling();
        let body = Body::read_cursor(cursor)?;

        assert!(!cursor.goto_next_sibling(), "Unexpected token in main");

        cursor.goto_parent();

        Ok(MainDecl { body, span })
    }
}

impl ReadCursor for TestDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>) -> Result<Self, AstError> {
        let test = cursor_expect!(cursor, "test");
        let span = Span::from_node(test);
        cursor.goto_first_child();
        cursor_expect!(cursor, "test_keyword");

        cursor.goto_next_sibling();
        let name = if cursor.kind()? == "string_literal" {
            let lit = Some(StringLiteral::read_cursor(cursor)?);
            cursor.goto_next_sibling();
            lit
        } else { None };   

        cursor.goto_next_sibling();
        let body = Body::read_cursor(cursor)?;

        cursor.goto_parent();

        Ok(TestDecl { name, body })
    }
}

impl ReadCursor for TypeDecl {}

impl ReadCursor for FnDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>) -> Result<Self, AstError> {
        let function = cursor_expect!(cursor, "function");
        let span = Span::from_node(function);
        cursor.goto_first_child();
        
        let signature = FnSignature::read_cursor(cursor);

        cursor.goto_next_sibling();
        let body = Body::read_cursor(cursor)?;

        cursor.goto_parent();
        
        Ok(FnDecl { signature, body, span })
    }
}

impl ReadCursor for FnSignature {
    fn read_cursor(cursor: &mut TreeCursor<'_>) -> Result<Self, AstError> {
        let signature = cursor_expect!(cursor, "fn_signature");
        let span = Span::from_node(signature);
        let visibility = if cursor.kind()? == "visibility" {
            let vis = Some(Visibility::read_cursor(cursor)?);
            cursor.goto_next_sibling();
            vis
        };

        cursor_expect!(cursor, "fn_keyword");

        cursor.goto_next_sibling();
        let identifier = Ident::read_cursor(cursor)?;
        
        cursor.goto_next_sibling();
        let parameters = ParamList::read_cursor(cursor)?;
        
        let return_type = if cursor.kind()? == "return_type" {
            cursor.goto_first_child();
            cursor_expect!(cursor, "single_arrow");
            cursor.goto_next_sibling();
            let ty = Some(TypeElement::read_cursor(cursor)?);
            cursor.goto_parent();
            ty
        } else { None };

        cursor.goto_parent();

        Ok(FnSignature { visibility, identifier, parameters, return_type, span })
    }
}
