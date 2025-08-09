use crate::context::Context;
use crate::{Transpile, Visibility};
use galvan_ast::VisibilityKind;
use galvan_resolver::Scope;

impl Transpile for Visibility {
    fn transpile(&self, _: &Context, _scope: &mut Scope, _errors: &mut crate::ErrorCollector) -> String {
        match self.kind {
            VisibilityKind::Public => "pub",
            VisibilityKind::Private => "",
            VisibilityKind::Inherited => "pub(crate)",
        }
        .into()
    }
}
