use galvan_ast::{Span, Visibility, VisibilityKind};
use galvan_parse::TreeCursor;

use crate::result::CursorUtil;
use crate::{AstError, ReadCursor};

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
