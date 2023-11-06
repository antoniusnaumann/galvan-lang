use galvan_parser::Visibility;

use crate::Transpile;

impl Transpile for Visibility {
    fn transpile(self) -> String {
        match self {
            Visibility::Public => "pub",
            Visibility::Private => "",
            Visibility::Inherited => "pub(crate)",
        }
        .into()
    }
}
