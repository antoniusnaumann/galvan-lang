use galvan_ast::Visibility;
use galvan_parse::TreeCursor;

use crate::result::CursorUtil;
use crate::AstError;

pub(crate) fn read_visibility(
    cursor: &mut TreeCursor,
    source: &str,
) -> Result<Option<Visibility>, AstError> {
    Ok(if cursor.kind()? == "visibility" {
        let vis = Some(Visibility::read_cursor(cursor, source)?);
        cursor.goto_next_sibling();
        vis
    } else {
        None
    })
}
