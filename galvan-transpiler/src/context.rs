use crate::mapping::Mapping;
use galvan_ast::{EmptyTypeDecl, SegmentedAsts, ToplevelItem, TypeDecl, Visibility};
use galvan_files::Source;
use galvan_resolver::{LookupContext, LookupError};

#[derive(Debug, Default)]
pub struct Context<'a> {
    pub lookup: LookupContext<'a>,
    pub mapping: Mapping,
    // pub scope: Scope,
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

pub fn predefined_from(mapping: &Mapping) -> SegmentedAsts {
    let types = mapping
        .types
        .iter()
        .map(|(ident, rust_type)| ToplevelItem {
            item: TypeDecl::Empty(EmptyTypeDecl {
                visibility: Visibility::Inherited,
                ident: ident.clone(),
            }),
            source: Source::Missing,
        })
        .collect();
    let functions = vec![];
    let tests = vec![];
    let main = None;
    SegmentedAsts {
        types,
        functions,
        tests,
        main,
    }
}
