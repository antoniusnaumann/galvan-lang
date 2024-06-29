#[macro_export]
macro_rules! cursor_expect {
    ($cursor:ident, $rule:literal) => {
        {
            use crate::result::CursorUtil;
            let node = $cursor.curr()?;
            if node.kind() != $rule {
                let kind = node.kind();
                $cursor.goto_parent();
                let parent = $cursor.curr()?;
                unreachable!("Expected {} keyword, got: {}, in: {}", $rule, kind, parent.kind());
            } else {
                node
            }
        }
    };
}
