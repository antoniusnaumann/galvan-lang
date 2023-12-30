use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::{FnDecl, FnSignature, Param, ParamList, Transpile};
use galvan_ast::DeclModifier;

impl_transpile!(FnDecl, "{} {}", signature, block);

impl Transpile for FnSignature {
    fn transpile(&self, ctx: &Context) -> String {
        let visibility = self.visibility.transpile(ctx);
        let identifier = self.identifier.transpile(ctx);
        let parameters = self.parameters.transpile(ctx);
        format!(
            "{} fn {}{}{}",
            visibility,
            identifier,
            parameters,
            self.return_type
                .as_ref()
                .map_or("".into(), |return_type| transpile!(
                    ctx,
                    " -> {}",
                    return_type
                ))
        )
    }
}

impl_transpile!(ParamList, "({})", params);

impl Transpile for Param {
    fn transpile(&self, ctx: &Context) -> String {
        let is_self = self.identifier.as_str() == "self";

        match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Inherited => {
                if is_self {
                    "&self".into()
                } else {
                    transpile!(ctx, "{}: &{}", self.identifier, self.param_type)
                }
            }
            DeclModifier::Mut => {
                if is_self {
                    "&mut self".into()
                } else {
                    transpile!(ctx, "{}: &mut {}", self.identifier, self.param_type)
                }
            }
            DeclModifier::Ref => {
                if is_self {
                    panic!("Functions with ref-receivers should be handled elsewhere!")
                }

                transpile!(
                    ctx,
                    "{}: std::sync::Arc<std::sync::Mutex<{}>>",
                    self.identifier,
                    self.param_type
                )
            }
        }
    }
}
