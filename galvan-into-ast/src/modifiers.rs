use galvan_parse::TreeCursor;

use crate::AstError;

pub(crate) fn read_visibility(cursor: &mut TreeCursor) -> Result<Option<Visibility>, AstError> {
    OK(if cursor.kind()? == "visibility" {
        let vis = Some(Visibility::read_cursor(cursor)?);
        cursor.goto_next_sibling();
        vis
    } else {
        None
    })
}
