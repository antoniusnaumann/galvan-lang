use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_match, transpile};
use crate::Transpile;
use galvan_ast::{
    ConstructorCall, ConstructorCallArg, DeclModifier, Expression, FunctionCall, FunctionCallArg,
    Ident, MemberFieldAccess, MemberFunctionCall,
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

impl Transpile for FunctionCallArg {
    fn transpile(&self, ctx: &Context) -> String {
        use DeclModifier as Mod;
        use Expression as Exp;
        let Self {
            modifier,
            expression,
        } = self;
        match (modifier, expression) {
            (Mod::Let, _) => {
                todo!("TRANSPILER ERROR: Let modifier is not allowed for function call arguments")
            }
            (Mod::Inherited, expression) => {
                transpile!(ctx, "&({})", expression)
            }
            (Mod::Mut, expr @ Exp::MemberFieldAccess(_) | expr @ Exp::Ident(_)) => {
                transpile!(ctx, "&mut {}", expr)
            }
            (Mod::Ref, expr @ Exp::MemberFieldAccess(_) | expr @ Exp::Ident(_)) => {
                transpile!(ctx, "::std::sync::Arc::clone(&{})", expr)
            }
            _ => todo!("TRANSPILER ERROR: Modifier only allowed for fields or variables"),
        }
    }
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
