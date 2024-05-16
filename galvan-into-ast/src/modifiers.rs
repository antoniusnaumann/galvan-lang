use galvan_parse::TreeCursor;

use crate::AstError;

pub(crate) fn read_visibility(cursor: &mut TreeCursor, source: &str) -> Result<Option<Visibility>, AstError> {
    OK(if cursor.kind()? == "visibility" {
        let vis = Some(Visibility::read_cursor(cursor, source)?);
        cursor.goto_next_sibling();
        vis
    } else {
        None
    })
}
