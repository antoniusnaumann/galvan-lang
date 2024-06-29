use galvan_ast::{DeclModifier, Span, Visibility, VisibilityKind};
use galvan_parse::TreeCursor;

use crate::result::CursorUtil;
use crate::{cursor_expect, AstError, ReadCursor};

impl ReadCursor for Visibility {
    fn read_cursor(cursor: &mut TreeCursor, source: &str) -> Result<Visibility, AstError> {
        Ok(if cursor.kind()? == "visibility" {
            let vis = Visibility::read_cursor(cursor, source)?;
            cursor.goto_next_sibling();
            vis
        } else {
            Visibility::new(
                VisibilityKind::Inherited,
                // TODO: Get end range from previous token here instead
                Span::default(),
            )
        })
    }
}

impl ReadCursor for DeclModifier {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "declaration_modifier");

        cursor.goto_first_child();

        let modifier = match cursor.kind()? {
            "ref_keyword" => Self::Ref,
            "let_keyword" => Self::Let,
            "mut_keyword" => Self::Mut,
            _ => unreachable!("Unexpected declaration modifier!")
        };

        cursor.goto_parent();

        Ok(modifier)
    }
}
