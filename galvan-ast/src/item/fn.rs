use super::*;

#[derive(Debug, PartialEq, Eq)]
pub struct FnDecl {
    // pub annotations,
    pub signature: FnSignature,
    pub block: Body,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnSignature {
    // pub asyncness: Async,
    // pub constness: Const,
    pub visibility: Visibility,
    pub identifier: Ident,
    pub parameters: ParamList,
    pub return_type: Option<TypeElement>,
}

impl FnSignature {
    pub fn new(
        mods: Modifiers,
        ident: Ident,
        parameters: ParamList,
        return_type: Option<TypeElement>,
    ) -> Self {
        // TODO: Verify that only first parameter is named self (or no self exists)
        FnSignature {
            // asyncness: mods.asyncness,
            // constness: mods.constness,
            visibility: mods.visibility,
            identifier: ident,
            parameters,
            return_type,
        }
    }

    pub fn receiver(&self) -> Option<&Param> {
        self.parameters
            .params
            .first()
            .filter(|param| param.identifier.as_str() == "self")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamList {
    pub params: Vec<Param>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub decl_modifier: Option<DeclModifier>,
    pub identifier: Ident,
    pub param_type: TypeElement,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DeclModifier {
    Let,
    Mut,
    Ref,
}
