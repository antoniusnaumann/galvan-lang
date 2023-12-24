use crate::macros::{impl_transpile, transpile};
use crate::{FnDecl, FnSignature, LookupContext, Param, ParamList, Transpile};

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
impl_transpile!(Param, "{}: {}", identifier, param_type);
