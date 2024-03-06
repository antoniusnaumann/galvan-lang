#[macro_export]
macro_rules! cursor_expect {
    ($cursor:ident, $rule:literal) => {
        {
            let node = $cursor.curr()?;
            if node != $rule {
                unreachable!("Expected {} keyword, got: {}", $rule, $rule);
            }
            node
        }
    };
}
