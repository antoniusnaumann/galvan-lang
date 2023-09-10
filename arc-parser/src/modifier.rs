#[derive(Debug)]
pub enum Visibility {
    Public,
    Private,
    // Inherited usually means pub(crate)
    Inherited,
}
