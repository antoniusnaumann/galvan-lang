use crate::macros::{impl_transpile, impl_transpile_variants, transpile};
use crate::{Block, Transpile};

use galvan_ast::{
    Assignment, DeclModifier, Declaration, Expression, FunctionCall, NumberLiteral, Statement,
    StringLiteral,
};
use galvan_resolver::LookupContext;

impl_transpile!(Block, "{{\n{}\n}}", statements);
impl_transpile_variants!(Statement; Assignment, Expression, Declaration);

impl Transpile for Declaration {
    fn transpile(&self, lookup: &LookupContext) -> String {
        let keyword = match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Ref => "let",
            DeclModifier::Mut => "let mut",
            DeclModifier::Inherited => panic!("Inherited declaration modifier is not allowed here"),
        };

        let identifier = self.identifier.transpile(lookup);
        let ty = self
            .type_annotation
            .as_ref()
            .map(|ty| transpile!(lookup, "{}", ty));
        let ty = match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Mut => ty.map_or("".into(), |ty| format!(": {ty}")),
            DeclModifier::Ref => {
                format!(
                    ": std::sync::Arc<std::sync::Mutex<{}>>",
                    ty.unwrap_or("_".into()),
                )
            }
            DeclModifier::Inherited => unreachable!(),
        };

        let expression = self
            .expression
            .as_ref()
            .map(|expr| transpile!(lookup, " = {}", expr))
            .unwrap_or("".into());

        format!("{keyword} {identifier}{ty}{expression}")
    }
}

// TODO: Check if variable is in scope
// TODO: Handle cloning
impl_transpile!(Assignment, "{} = {}", identifier, expression);

impl_transpile_variants!(Expression; StringLiteral, NumberLiteral, FunctionCall, Ident);
impl Transpile for StringLiteral {
    fn transpile(&self, _lookup: &LookupContext) -> String {
        // TODO: Implement more sophisticated formatting (extract {} and put them as separate arguments)
        format!("format!({})", self.as_str())
    }
}

impl Transpile for FunctionCall {
    fn transpile(&self, lookup: &LookupContext) -> String {
        // TODO: Resolve function and check argument types + check if they should be submitted as &, &mut or Arc<Mutex>
        let identifier = self.identifier.transpile(lookup);
        let arguments = self
            .arguments
            .iter()
            // TODO: Check if reference or mut reference is needed (or no reference at all for copy types)
            .map(|arg| format!("&{}", arg.transpile(lookup)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({})", identifier, arguments)
    }
}

impl Transpile for NumberLiteral {
    fn transpile(&self, lookup: &LookupContext) -> String {
        // TODO: Parse number and validate type
        format!("{}", self.as_str())
    }
}
