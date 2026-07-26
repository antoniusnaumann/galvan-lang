use galvan_ast::SegmentedAsts;
use galvan_hir::mapping::Mapping;
use galvan_resolver::LookupContext;

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

    /// Registers `asts` in the lookup context. Duplicate declarations have
    /// already been reported by the typechecker at this point, so conflicts
    /// are resolved the same way (first declaration wins).
    pub fn with(mut self, asts: &'a SegmentedAsts) -> Self {
        self.lookup = self.lookup.with(asts);
        self
    }
}
