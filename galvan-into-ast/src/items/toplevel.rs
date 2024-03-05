use galvan_ast::{FnDecl, MainDecl, RootItem, TestDecl, TypeDecl};
use galvan_parse::TreeCursor;

use crate::{result::CursorUtil, AstError, ReadCursor};

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

impl ReadCursor for MainDecl {}

impl ReadCursor for TestDecl {}

impl ReadCursor for TypeDecl {}

impl ReadCursor for FnDecl {}
