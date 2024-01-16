use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::Transpile;
use galvan_ast::{
    ConstructorCall, ConstructorCallArg, DeclModifier, Expression, FunctionCall, FunctionCallArg,
    MemberFieldAccess, MemberFunctionCall, Ownership,
};
use galvan_resolver::Scope;
use itertools::Itertools;

impl Transpile for FunctionCall {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        match self.identifier.as_str() {
            "panic" => format!("panic!(\"{{}}\", {})", self.arguments.transpile(ctx, scope)),
            "println" => format!(
                "println!(\"{{}}\", {})",
                self.arguments.transpile(ctx, scope)
            ),
            "print" => format!("print!(\"{{}}\", {})", self.arguments.transpile(ctx, scope)),
            "debug" => format!(
                "println!(\"{{:?}}\", {})",
                self.arguments.transpile(ctx, scope)
            ),
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
                                scope,
                                "assert_eq!({}, {}, {})",
                                left,
                                right,
                                args.transpile(ctx, scope)
                            )
                        }
                        galvan_ast::ComparisonOperator::NotEqual => {
                            transpile!(
                                ctx,
                                scope,
                                "assert_ne!({}, {}, {})",
                                left,
                                right,
                                args.transpile(ctx, scope)
                            )
                        }
                        _ => format!("assert!({})", self.arguments.transpile(ctx, scope)),
                    }
                }
                Some(_) => format!("assert!({})", self.arguments.transpile(ctx, scope)),
                _ => todo!("TRANSPILER ERROR: assert expects a boolean argument"),
            },
            _ => {
                // TODO: Resolve function and check argument types + check if they should be submitted as &, &mut or Arc<Mutex>
                let ident = self.identifier.transpile(ctx, scope);
                format!("{}({})", ident, self.arguments.transpile(ctx, scope))
            }
        }
    }
}

impl Transpile for FunctionCallArg {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
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
            (None, Exp::Ident(ident)) => {
                match scope
                    .get_variable(ident)
                    .unwrap_or_else(|| {
                        panic!(
                            "TODO: ERROR: undeclared variable {ident}, scope: {:#?}",
                            scope
                        )
                    })
                    .ownership
                {
                    Ownership::Owned => {
                        transpile!(ctx, scope, "&{}", ident)
                    }
                    Ownership::Borrowed | Ownership::MutBorrowed | Ownership::Copy => {
                        transpile!(ctx, scope, "{}", ident)
                    }
                    Ownership::Ref => {
                        transpile!(ctx, scope, "{}.lock().unwrap()", ident)
                    }
                }
            }
            (None, Exp::Closure(closure)) => {
                transpile!(ctx, scope, "{}", closure)
            }
            (None, expression) => {
                transpile!(ctx, scope, "&({})", expression)
            }
            (Some(Mod::Mut(_)), expr @ Exp::MemberFieldAccess(_) | expr @ Exp::Ident(_)) => {
                transpile!(ctx, scope, "&mut {}", expr)
            }
            (Some(Mod::Ref(_)), expr @ Exp::MemberFieldAccess(_) | expr @ Exp::Ident(_)) => {
                transpile!(ctx, scope, "::std::sync::Arc::clone(&{})", expr)
            }
            _ => todo!("TRANSPILER ERROR: Modifier only allowed for fields or variables"),
        }
    }
}

impl Transpile for MemberFunctionCall {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self {
            receiver,
            identifier,
            arguments,
        } = self;

        let receiver_chain = transpile_receiver_chain(ctx, scope, receiver);
        transpile!(ctx, scope, "{receiver_chain}.{}({})", identifier, arguments)
    }
}

impl Transpile for MemberFieldAccess {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self {
            receiver,
            identifier,
        } = self;

        let chain = transpile_receiver_chain(ctx, scope, receiver);
        transpile!(ctx, scope, "{chain}.{}", identifier)
    }
}

fn transpile_receiver_chain(ctx: &Context, scope: &mut Scope, receiver: &[Expression]) -> String {
    receiver
        .iter()
        .map(|r| transpile!(ctx, scope, "{}", r))
        .collect_vec()
        .join(".")
}

impl_transpile!(ConstructorCall, "{} {{ {} }}", identifier, arguments,);
impl_transpile!(ConstructorCallArg, "{}: {}", ident, expression);
