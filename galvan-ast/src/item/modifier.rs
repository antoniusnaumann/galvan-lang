use galvan_pest::Rule;

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

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::visibility))]
pub enum Visibility {
    // Inherited usually means pub(crate)
    #[default]
    Inherited,
    Public(Pub),
    // Private,
}

impl Visibility {
    pub fn public() -> Self {
        Self::Public(Pub)
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::pub_keyword))]
pub struct Pub;

#[derive(Clone, Copy, Debug, PartialEq, Eq,)]
pub enum Ownership {
    Val,
    StoredRef,
    BorrowedRef,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq,)]
pub enum Async {
    Async,
    // This usually means sync
    #[default]
    Inherited,
    // This is not implemented but will be supported in future versions
    Generic,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq,)]
pub enum Const {
    Const,
    // This usually means not const
    #[default]
    Inherited,
    // This is not implemented but will be supported in future versions
    Generic,
}
