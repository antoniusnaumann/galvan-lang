use derive_more::Display;

#[derive(Debug, Display, PartialEq, Eq)]
pub struct Ident(String);

impl Ident {
    pub fn new<S: ToOwned<Owned = String>>(name: S) -> Ident {
        Ident(name.to_owned())
    }
}
