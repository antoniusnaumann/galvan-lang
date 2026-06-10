use crate::context::Context;
use crate::Transpile;
use galvan_ast::{Visibility, VisibilityKind};

impl Transpile for Visibility {
    fn transpile(&self, _: &Context, _errors: &mut crate::ErrorCollector) -> String {
        match self.kind {
            VisibilityKind::Public => "pub",
            VisibilityKind::Private => "",
            VisibilityKind::Inherited => "pub(crate)",
        }
        .into()
    }
}
