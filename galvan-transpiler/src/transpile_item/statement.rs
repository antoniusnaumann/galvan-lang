use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_match, impl_transpile_variants, transpile};
use crate::{Block, Transpile};
use galvan_ast::{
    DeclModifier, Declaration, Expression, FunctionCall, FunctionCallArg, IdentArg,
    MemberFieldAccess, MemberFunctionCall, NumberLiteral, Statement, StringLiteral,
};

impl_transpile!(Block, "{{\n{}\n}}", statements);
impl_transpile_variants!(Statement; Assignment, Expression, Declaration);

impl Transpile for Declaration {
    fn transpile(&self, ctx: &Context) -> String {
        let keyword = match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Ref => "let",
            DeclModifier::Mut => "let mut",
            DeclModifier::Inherited => panic!("Inherited declaration modifier is not allowed here"),
        };

        let identifier = self.identifier.transpile(ctx);
        let ty = self
            .type_annotation
            .as_ref()
            .map(|ty| transpile!(ctx, "{}", ty));
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

        // TODO: Wrap non-ref types in Arc<Mutex<>> when assigned to a ref type, clone ref types
        // TODO: Clone inner type from ref types to non-ref types
        self.expression
            .as_ref()
            .map(|expr| transpile_assignment_expression(ctx, keyword, expr))
            .map(|expr| format!("{keyword} {identifier}{ty} = {expr}"))
            .unwrap_or_else(|| format!("{keyword} {identifier}{ty}"))
    }
}

fn transpile_assignment_expression(ctx: &Context, keyword: &str, expr: &Expression) -> String {
    match expr {
        Expression::Ident(ident) => {
            transpile!(ctx, "{}.clone()", ident)
        }
        expr => expr.transpile(ctx),
    }
}

impl_transpile_variants!(Expression; StringLiteral, NumberLiteral, FunctionCall, Ident);
impl Transpile for StringLiteral {
    fn transpile(&self, _: &Context) -> String {
        // TODO: Implement more sophisticated formatting (extract {} and put them as separate arguments)
        format!("format!({})", self.as_str())
    }
}

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
                transpile!(ctx, "&{}", self.ident)
            }
            DeclModifier::Mut => {
                transpile!(ctx, "&mut {}", self.ident)
            }
            DeclModifier::Ref => {
                transpile!(ctx, "::std::sync::Arc::clone(&{})", self.ident)
            }
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

impl Transpile for NumberLiteral {
    fn transpile(&self, _: &Context) -> String {
        // TODO: Parse number and validate type
        format!("{}", self.as_str())
    }
}
