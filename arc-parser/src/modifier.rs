#[derive(Debug)]
pub enum Visibility {
    Public,
    Private,
    // Inherited usually means pub(crate)
    Inherited,
}

#[derive(Debug)]
pub enum Ownership {
    Val,
    StoredRef,
    BorrowedRef,
}

pub enum Async {
    Async,
    // This usually means sync
    Inherited,
    // This is not implemented but will be supported in future versions
    Generic,
}

pub enum Const {
    Const,
    // This usually means not const
    Inherited,
    // This is not implemented but will be supported in future versions
    Generic,
}
