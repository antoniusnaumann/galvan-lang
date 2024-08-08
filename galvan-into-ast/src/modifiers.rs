use galvan_ast::{DeclModifier, Span, Visibility, VisibilityKind};
use galvan_parse::TreeCursor;

use crate::result::CursorUtil;
use crate::{cursor_expect, AstError, ReadCursor};

impl ReadCursor for Visibility {
    fn read_cursor(cursor: &mut TreeCursor, source: &str) -> Result<Visibility, AstError> {
        let kind = if cursor.kind()? == "visibility" {
            cursor.child();
            let vis = match cursor.kind()? {
                "pub_keyword" => VisibilityKind::Public,
                unknown => unreachable!("Unknown visibility modifier: {unknown}"),
            };
            cursor.goto_parent();

            cursor.next();
            vis
        } else {
            VisibilityKind::Inherited
        };

        Ok(Visibility::new(
            kind,
            // TODO: Get end range from previous token here instead
            Span::default(),
        ))
    }
}

impl ReadCursor for DeclModifier {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "declaration_modifier");

        cursor.child();

        let modifier = match cursor.kind()? {
            "ref_keyword" => Self::Ref,
            "let_keyword" => Self::Let,
            "mut_keyword" => Self::Mut,
            _ => unreachable!("Unexpected declaration modifier!"),
        };

        cursor.goto_parent();

        Ok(modifier)
    }
}
