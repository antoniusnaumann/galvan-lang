#[macro_export]
macro_rules! cursor_expect {
    ($cursor:ident, $rule:literal) => {
        {
            use crate::result::CursorUtil;
            let node = $cursor.curr()?;
            if node.kind() != $rule {
                unreachable!("Expected {} keyword, got: {}", $rule, node.kind());
            }
            node
        }
    };
}
