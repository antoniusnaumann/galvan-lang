use galvan_ast::{Ident, TypeIdent};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RustArgConversion {
    #[default]
    None,
    SharedBorrow,
    BoxNew,
    RcNew,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RustReturnConversion {
    #[default]
    None,
    BoxDeref,
    RcCloneDeref,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustFieldConversion {
    pub field: Ident,
    pub arg_conversion: RustArgConversion,
    pub return_conversion: RustReturnConversion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustEnumVariantConversion {
    pub variant: TypeIdent,
    pub args: Vec<RustEnumVariantArgConversion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustEnumVariantArgConversion {
    pub field: Option<Ident>,
    pub arg_conversion: RustArgConversion,
    pub return_conversion: RustReturnConversion,
}
