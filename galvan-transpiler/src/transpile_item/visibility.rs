use crate::context::Context;
use crate::{Transpile, Visibility};
use galvan_resolver::Scope;

impl Transpile for Visibility {
    fn transpile(&self, _: &Context, _scope: &mut Scope) -> String {
        match self {
            Visibility::Public(_) => "pub",
            Visibility::Private => "",
            Visibility::Inherited => "pub(crate)",
        }
        .into()
    }
}
