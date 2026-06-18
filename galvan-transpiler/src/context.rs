use galvan_ast::SegmentedAsts;
use galvan_hir::mapping::Mapping;
use galvan_resolver::{LookupContext, LookupError};

pub use galvan_hir::builtins::predefined_from;

#[derive(Debug, Default)]
pub struct Context<'a> {
    pub lookup: LookupContext<'a>,
    pub mapping: Mapping,
}

impl<'a> Context<'a> {
    pub fn new(mapping: Mapping) -> Self {
        Self {
            lookup: LookupContext::default(),
            mapping,
        }
    }

    pub fn with(mut self, asts: &'a SegmentedAsts) -> Result<Self, LookupError> {
        self.lookup = self.lookup.with(asts)?;
        Ok(self)
    }
}
