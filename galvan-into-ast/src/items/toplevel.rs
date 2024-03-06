use galvan_ast::{Body, FnDecl, MainDecl, RootItem, TestDecl, TypeDecl};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor};

impl ReadCursor for RootItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>) -> Result<Self, AstError> {
        Ok(match cursor.curr()? {
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
        cursor_expect!(cursor, "main");
        cursor.goto_first_child();
        cursor_expect!(cursor, "main_keyword");

        cursor.goto_next_sibling();
        let body_rule = cursor_expect!(cursor, "body");
        let body = Body::read_cursor(cursor)?;

        assert!(!cursor.goto_next_sibling(), "Unexpected token in main");

        cursor.goto_parent();

        Ok(MainDecl { body })
    }
}

impl ReadCursor for TestDecl {}

impl ReadCursor for TypeDecl {}

impl ReadCursor for FnDecl {}
