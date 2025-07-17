use crate::mapping::Mapping;
use galvan_ast::{
    AstNode, EmptyTypeDecl, FnDecl, SegmentedAsts, ToplevelItem, TypeDecl, Visibility,
    VisibilityKind,
};
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

pub fn predefined_from(mapping: &Mapping, functions: Vec<FnDecl>) -> SegmentedAsts {
    let types = mapping
        .types
        .keys()
        .map(|ident| ToplevelItem {
            item: TypeDecl::Empty(EmptyTypeDecl {
                visibility: Visibility::new(VisibilityKind::Inherited, ident.span().clone()),
                ident: ident.clone(),
                span: ident.span().clone(),
            }),
            source: Source::Missing,
        })
        .collect();
    let tests = vec![];
    let functions = functions
        .into_iter()
        .map(|f| ToplevelItem {
            item: f,
            source: Source::Builtin,
        })
        .collect();
    let main = None;
    SegmentedAsts {
        types,
        functions,
        tests,
        main,
    }
}
