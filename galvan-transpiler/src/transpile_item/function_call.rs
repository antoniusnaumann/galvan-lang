use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_match, transpile};
use crate::Transpile;
use galvan_ast::{
    ConstructorCall, ConstructorCallArg, DeclModifier, FunctionCall, FunctionCallArg, Ident,
    IdentArg, MemberFieldAccess, MemberFunctionCall,
};

impl Transpile for FunctionCall {
    fn transpile(&self, ctx: &Context) -> String {
        let arguments = self.arguments.transpile(ctx);

        // TODO: Resolve function and check argument types + check if they should be submitted as &, &mut or Arc<Mutex>
        if self.identifier.as_str() == "println" {
            format!("println!(\"{{}}\", {})", arguments)
        } else if self.identifier.as_str() == "print" {
            format!("print!(\"{{}}\", {})", arguments)
        } else {
            let ident = self.identifier.transpile(ctx);
            format!("{}({})", ident, arguments)
        }
    }
}

impl_transpile_match! { FunctionCallArg,
   Ident(arg) => ("{}", arg),
   Expr(expr) => ("&({})", expr),
}

impl Transpile for IdentArg {
    fn transpile(&self, ctx: &Context) -> String {
        match self.modifier {
            DeclModifier::Let => {
                panic!("Let modifier is not allowed for function call arguments")
            }
            DeclModifier::Inherited => {
                transpile!(ctx, "&{}", ident_chain(ctx, &self.dotted))
            }
            DeclModifier::Mut => {
                transpile!(ctx, "&mut {}", ident_chain(ctx, &self.dotted))
            }
            DeclModifier::Ref => {
                transpile!(
                    ctx,
                    "::std::sync::Arc::clone(&{})",
                    ident_chain(ctx, &self.dotted)
                )
            }
        }
    }
}

fn ident_chain(ctx: &Context, idents: &[Ident]) -> String {
    idents
        .iter()
        .map(|ident| ident.transpile(ctx))
        .collect::<Vec<_>>()
        .join(".")
}

impl_transpile!(
    MemberFunctionCall,
    "{}.{}({})",
    receiver,
    identifier,
    arguments
);
impl_transpile!(MemberFieldAccess, "{}.{}", receiver, identifier);

impl_transpile!(ConstructorCall, "{} {{ {} }}", identifier, arguments,);
impl_transpile!(ConstructorCallArg, "{}: {}", ident, expression);
