use crate::{Transpile, Visibility};
use crate::context::Context;

impl Transpile for Visibility {
    fn transpile(&self, _: &Context) -> String {
        match self {
            Visibility::Public(_) => "pub",
            // Visibility::Private => "",
            Visibility::Inherited => "pub(crate)",
        }
        .into()
    }
}
