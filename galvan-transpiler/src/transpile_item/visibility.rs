use crate::{Transpile, Visibility};

impl Transpile for Visibility {
    fn transpile(self) -> String {
        match self {
            Visibility::Public(_) => "pub",
            // Visibility::Private => "",
            Visibility::Inherited => "pub(crate)",
        }
        .into()
    }
}
