use std::borrow::Borrow;
use crate::builtins::BORROWED_ITERATOR_FNS;
use crate::context::Context;
use crate::macros::transpile;
use crate::transpile_item::closure::transpile_closure;
use crate::transpile_item::statement::match_ident;
use crate::type_inference::InferType;
use crate::Transpile;
use galvan_ast::TypeElement::Plain;
use galvan_ast::{
    ComparisonOperator, DeclModifier, Expression, FunctionCall, FunctionCallArg, InfixExpression,
    InfixOperation, Ownership,
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
                    expression: Expression::Infix(e),
                    span,
                }) if e.is_comparison() => {
                    if modifier.is_some() {
                        todo!("TRANSPILER ERROR: assert modifier is not allowed for comparison operations")
                    }

                    let InfixExpression::Comparison(comp) = e.borrow() else { unreachable!() };

                    let InfixOperation { lhs, operator, rhs, span } = comp;
                    let args = if self.arguments.len() > 1 {
                        &self.arguments[1..]
                    } else {
                        &[]
                    };
                    match operator {
                        ComparisonOperator::Equal => {
                            transpile!(
                                ctx,
                                scope,
                                "assert_eq!({}, {}, {})",
                                lhs,
                                rhs,
                                args.transpile(ctx, scope)
                            )
                        }
                        ComparisonOperator::NotEqual => {
                            transpile!(
                                ctx,
                                scope,
                                "assert_ne!({}, {}, {})",
                                lhs,
                                rhs,
                                args.transpile(ctx, scope)
                            )
                        }
                        _ => format!("assert!({})", self.arguments.transpile(ctx, scope)),
                    }
                }
                Some(_) => format!("assert!({})", self.arguments.transpile(ctx, scope)),
                _ => todo!(
                    "TRANSPILER ERROR: assert expects a boolean argument, found: {:#?}",
                    self.arguments
                ),
            },
            s if BORROWED_ITERATOR_FNS.contains(&s) => {
                let ident = self.identifier.transpile(ctx, scope);
                let args = self
                    .arguments
                    .iter()
                    .map(|a| match &a.expression {
                        Expression::Closure(closure) => {
                            assert!(
                            a.modifier.is_none(),
                            "TRANSPILER ERROR: closure modifier not allowed for iterator functions"
                        );
                            transpile_closure(ctx, scope, closure, true)
                        }
                        _ => a.transpile(ctx, scope),
                    })
                    .join(", ");
                format!("{}({})", ident, args)
            }
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
            span,
        } = self;
        match (modifier, expression) {
            (Some(Mod::Let), _) => {
                todo!("TRANSPILER ERROR: Let modifier is not allowed for function call arguments")
            }
            (None, match_ident!(ident)) => {
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
                let t = expression.infer_type(scope);
                if t.is_some_and(|t| {
                    if let Plain(plain) = t {
                        ctx.mapping.is_copy(&plain.ident)
                    } else {
                        false
                    }
                }) {
                    transpile!(ctx, scope, "{}", expression)
                } else {
                    transpile!(ctx, scope, "&({})", expression)
                }
            }
            // TODO: Check if the infix expression is a member field access
            (
                Some(Mod::Mut),
                expr @ Exp::Infix(_) | expr @ match_ident!(_),
            ) => {
                transpile!(ctx, scope, "&mut {}", expr)
            }
            (
                Some(Mod::Ref),
                expr @ Exp::Infix(_) | expr @ match_ident!(_),
            ) => {
                transpile!(ctx, scope, "::std::sync::Arc::clone(&{})", expr)
            }
            _ => todo!("TRANSPILER ERROR: Modifier only allowed for fields or variables"),
        }
    }
}
