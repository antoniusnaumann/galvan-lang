use galvan_ast::{
    EmptyTypeDecl, EnumTypeMember, EnumVariantField, FnDecl, Ident, Param, StructTypeMember,
    TupleTypeMember, TypeDecl, TypeElement, TypeIdent, Visibility,
};

use crate::model::{
    RustArgConversion, RustEnumVariantArgConversion, RustEnumVariantConversion,
    RustFieldConversion, RustReturnConversion,
};

#[derive(Clone, Debug)]
pub(super) struct ImportedFunctionDecl {
    pub(super) decl: FnDecl,
    pub(super) return_conversion: RustReturnConversion,
    pub(super) arg_conversions: Vec<RustArgConversion>,
}

#[derive(Debug)]
pub(super) struct ImportedTypeDecl {
    pub(super) decl: TypeDecl,
    pub(super) field_conversions: Vec<RustFieldConversion>,
    pub(super) constructor_arg_conversions: Vec<RustArgConversion>,
    pub(super) enum_variant_conversions: Vec<RustEnumVariantConversion>,
}

impl ImportedTypeDecl {
    pub(super) fn new(decl: TypeDecl) -> Self {
        Self {
            decl,
            field_conversions: Vec::new(),
            constructor_arg_conversions: Vec::new(),
            enum_variant_conversions: Vec::new(),
        }
    }

    pub(super) fn empty(name: &str) -> Self {
        Self::empty_with_generics(name, Vec::new())
    }

    pub(super) fn empty_with_generics(name: &str, generic_params: Vec<Ident>) -> Self {
        Self::new(TypeDecl::Empty(EmptyTypeDecl {
            visibility: Visibility::public(),
            ident: TypeIdent::new(name),
            generic_params,
            span: galvan_ast::Span::default(),
        }))
    }
}

#[derive(Debug)]
pub(super) struct LiftedStructMember {
    pub(super) member: StructTypeMember,
    pub(super) arg_conversion: RustArgConversion,
    pub(super) return_conversion: RustReturnConversion,
}

#[derive(Debug)]
pub(super) struct LiftedTupleMember {
    pub(super) member: TupleTypeMember,
    pub(super) arg_conversion: RustArgConversion,
}

#[derive(Debug)]
pub(super) struct LiftedEnumMember {
    pub(super) member: EnumTypeMember,
    pub(super) arg_conversions: Vec<RustEnumVariantArgConversion>,
}

#[derive(Debug)]
pub(super) struct LiftedEnumVariantField {
    pub(super) field: EnumVariantField,
    pub(super) arg_conversion: RustArgConversion,
    pub(super) return_conversion: RustReturnConversion,
}

#[derive(Clone, Debug)]
pub(super) struct LiftedReturn {
    pub(super) ty: TypeElement,
    pub(super) decl_modifier: Option<galvan_ast::DeclModifier>,
    pub(super) return_conversion: RustReturnConversion,
}

#[derive(Clone, Debug)]
pub(super) struct LiftedParam {
    pub(super) param: Param,
    pub(super) arg_conversion: RustArgConversion,
}

#[derive(Clone, Debug)]
pub(super) struct LiftedType {
    pub(super) ty: TypeElement,
    pub(super) decl_modifier: Option<galvan_ast::DeclModifier>,
    pub(super) arg_conversion: RustArgConversion,
}

impl LiftedType {
    pub(super) fn new(ty: TypeElement) -> Self {
        Self {
            ty,
            decl_modifier: None,
            arg_conversion: RustArgConversion::None,
        }
    }

    pub(super) fn with_modifier(ty: TypeElement, decl_modifier: galvan_ast::DeclModifier) -> Self {
        Self {
            ty,
            decl_modifier: Some(decl_modifier),
            arg_conversion: RustArgConversion::None,
        }
    }
}
