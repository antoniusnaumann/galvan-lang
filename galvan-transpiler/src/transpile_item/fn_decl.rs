use crate::macros::{impl_transpile, transpile};
use crate::{FnDecl, FnSignature, LookupContext, Param, ParamList, Transpile};
use galvan_ast::DeclModifier;

impl_transpile!(FnDecl, "{} {}", signature, block);

impl Transpile for FnSignature {
    fn transpile(&self, lookup: &LookupContext) -> String {
        let visibility = self.visibility.transpile(lookup);
        let identifier = self.identifier.transpile(lookup);
        let parameters = self.parameters.transpile(lookup);
        format!(
            "{} fn {}{}{}",
            visibility,
            identifier,
            parameters,
            self.return_type
                .as_ref()
                .map_or("".into(), |return_type| transpile!(
                    lookup,
                    " -> {}",
                    return_type
                ))
        )
    }
}

impl_transpile!(ParamList, "({})", params);

impl Transpile for Param {
    fn transpile(&self, lookup: &LookupContext) -> String {
        let is_self = self.identifier.as_str() == "self";

        match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Inherited => {
                if is_self {
                    "&self".into()
                } else {
                    transpile!(lookup, "{}: &{}", self.identifier, self.param_type)
                }
            }
            DeclModifier::Mut => {
                if is_self {
                    "&mut self".into()
                } else {
                    transpile!(lookup, "{}: &mut {}", self.identifier, self.param_type)
                }
            }
            DeclModifier::Ref => {
                if is_self {
                    panic!("Functions with ref-receivers should be handled elsewhere!")
                }

                transpile!(
                    lookup,
                    "{}: std::sync::Arc<std::sync::Mutex<{}>>",
                    self.identifier,
                    self.param_type
                )
            }
        }
    }
}
