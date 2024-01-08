use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_match, transpile};
use crate::Transpile;
use galvan_ast::{
    ComparisonOperation, ConstructorCall, ConstructorCallArg, DeclModifier, Expression,
    FunctionCall, FunctionCallArg, MemberFieldAccess, MemberFunctionCall,
};

impl Transpile for FunctionCall {
    fn transpile(&self, ctx: &Context) -> String {
        match self.identifier.as_str() {
            "println" => format!("println!(\"{{}}\", {})", self.arguments.transpile(ctx)),
            "print" => format!("print!(\"{{}}\", {})", self.arguments.transpile(ctx)),
            "debug" => format!("println!(\"{{:?}}\", {})", self.arguments.transpile(ctx)),
            "assert" => match self.arguments.first() {
                Some(FunctionCallArg {
                    modifier,
                    expression: Expression::ComparisonOperation(comp),
                }) => {
                    if modifier.is_some() {
                        todo!("TRANSPILER ERROR: assert modifier is not allowed for comparison operations")
                    }

                    let ComparisonOperation {
                        left,
                        operator,
                        right,
                    } = comp;
                    let args = if self.arguments.len() > 1 {
                        &self.arguments[1..]
                    } else {
                        &[]
                    };
                    match operator {
                        galvan_ast::ComparisonOperator::Equal => {
                            transpile!(
                                ctx,
                                "assert_eq!({}, {}, {})",
                                left,
                                right,
                                args.transpile(ctx)
                            )
                        }
                        galvan_ast::ComparisonOperator::NotEqual => {
                            transpile!(
                                ctx,
                                "assert_ne!({}, {}, {})",
                                left,
                                right,
                                args.transpile(ctx)
                            )
                        }
                        _ => format!("assert!({})", self.arguments.transpile(ctx)),
                    }
                }
                Some(_) => format!("assert!({})", self.arguments.transpile(ctx)),
                _ => todo!("TRANSPILER ERROR: assert expects a boolean argument"),
            },
            _ => {
                // TODO: Resolve function and check argument types + check if they should be submitted as &, &mut or Arc<Mutex>
                let ident = self.identifier.transpile(ctx);
                format!("{}({})", ident, self.arguments.transpile(ctx))
            }
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
            (Some(Mod::Let(_)), _) => {
                todo!("TRANSPILER ERROR: Let modifier is not allowed for function call arguments")
            }
            (None, expr @ Exp::Ident(_)) => {
                transpile!(ctx, "(&{}).__borrow()", expr)
            }
            (None, expression) => {
                transpile!(ctx, "&({})", expression)
            }
            (Some(Mod::Mut(_)), expr @ Exp::MemberFieldAccess(_) | expr @ Exp::Ident(_)) => {
                transpile!(ctx, "&mut {}", expr)
            }
            (Some(Mod::Ref(_)), expr @ Exp::MemberFieldAccess(_) | expr @ Exp::Ident(_)) => {
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
