#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Visibility {
    // Inherited usually means pub(crate)
    #[default]
    Inherited,
    Public,
    Private,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ownership {
    Owned,
    Borrowed,
    MutBorrowed,
    Copy,
    Ref,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Async {
    Async,
    // This usually means sync
    #[default]
    Inherited,
    // This is not implemented but will be supported in future versions
    Generic,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Const {
    Const,
    // This usually means not const
    #[default]
    Inherited,
    // This is not implemented but will be supported in future versions
    Generic,
}
