use crate::context::Context;
use crate::error::ErrorCollector;
use crate::macros::impl_transpile_variants;
use crate::{Ast, RootItem, Transpile};
use galvan_resolver::Scope;
use convert_case::{Case, Casing};

impl Transpile for Ast {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        self.toplevel.transpile(ctx, scope, errors)
    }
}

impl_transpile_variants!(RootItem; Type, Fn, Main, Test, Cmd);

impl Transpile for galvan_ast::CmdDecl {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // Generate the command function signature and body
        let mut cmd_scope = Scope::child(scope).returns(galvan_ast::TypeElement::void(), galvan_ast::Ownership::UniqueOwned);
        let fn_signature = self.signature.transpile(ctx, &mut cmd_scope, errors);
        let fn_body = self.body.transpile(ctx, &mut cmd_scope, errors);
        
        format!("{} {}", fn_signature, fn_body)
    }
}

impl Transpile for galvan_ast::CmdSignature {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // For CLI commands, we transpose to a regular function signature
        // CLI commands are always private and return ()
        let identifier = self.identifier.transpile(ctx, scope, errors);
        
        // Convert parameters, ignoring short_name for the function signature
        let mut regular_params = Vec::new();
        for param in &self.parameters.params {
            let param_str = format!("{}: {}", param.identifier.transpile(ctx, scope, errors), param.param_type.transpile(ctx, scope, errors));
            regular_params.push(param_str);
        }
        let parameters = format!("({})", regular_params.join(", "));
        
        // CLI commands always have void return type
        format!("fn {}{}", identifier, parameters)
    }
}
