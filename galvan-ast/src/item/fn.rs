use super::*;
use derive_more::From;
use galvan_pest::Rule;

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::function))]
pub struct FnDecl {
    pub signature: FnSignature,
    pub block: Body,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::fn_signature))]
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

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::param_list))]
pub struct ParamList {
    pub params: Vec<Param>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::param))]
pub struct Param {
    pub decl_modifier: Option<DeclModifier>,
    pub identifier: Ident,
    pub param_type: TypeElement,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::declaration_modifier))]
pub enum DeclModifier {
    Let(LetKeyword),
    Mut(MutKeyword),
    Ref(RefKeyword),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::let_keyword))]
pub struct LetKeyword;

#[derive(Copy, Clone, Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::mut_keyword))]
pub struct MutKeyword;

#[derive(Copy, Clone, Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::ref_keyword))]
pub struct RefKeyword;
