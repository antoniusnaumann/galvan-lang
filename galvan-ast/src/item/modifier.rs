#[derive(Clone, Default, Debug)]
pub struct Modifiers {
    pub visibility: Visibility,
    pub constness: Const,
    pub asyncness: Async,
}

impl Modifiers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn has_vis_modifier(&self) -> bool {
        !matches!(self.visibility, Visibility::Inherited)
    }

    pub fn has_async_modifier(&self) -> bool {
        !matches!(self.asyncness, Async::Inherited)
    }

    pub fn has_const_modifier(&self) -> bool {
        !matches!(self.constness, Const::Inherited)
    }
}

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
