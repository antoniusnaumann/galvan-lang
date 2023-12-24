use crate::{LookupContext, Transpile, Visibility};

impl Transpile for Visibility {
    fn transpile(&self, _: &LookupContext) -> String {
        match self {
            Visibility::Public(_) => "pub",
            // Visibility::Private => "",
            Visibility::Inherited => "pub(crate)",
        }
        .into()
    }
}
