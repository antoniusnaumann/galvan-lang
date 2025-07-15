use crate::builtins::BORROWED_ITERATOR_FNS;
use crate::context::Context;
use crate::macros::transpile;
use crate::transpile_item::closure::transpile_closure;
use crate::transpile_item::statement::match_ident;
use crate::type_inference::InferType;
use crate::Transpile;
use galvan_ast::TypeElement::{self, Plain};
use galvan_ast::{
    Assignment, ComparisonOperator, DeclModifier, Expression, ExpressionKind, FunctionCall,
    FunctionCallArg, Ident, InfixExpression, InfixOperation, Ownership,
};
use galvan_resolver::Scope;
use itertools::Itertools;
use std::borrow::Borrow;

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
            "if" => {
                let ty = self.infer_type(scope);
                match ty {
                    Some(ty @ TypeElement::Optional(_)) => {
                        let if_ = transpile_if(self, ctx, scope, Some(ty));
                        format!("{if_} else {{ None }}")
                    }
                    ty => transpile_if(self, ctx, scope, ty),
                }
            }
            "for" => transpile_for(self, ctx, scope),
            "assert" => match self.arguments.first() {
                Some(FunctionCallArg {
                    modifier,
                    expression:
                        Expression {
                            kind: ExpressionKind::Infix(e),
                            span,
                            type_: _,
                        },
                }) if e.is_comparison() => {
                    if modifier.is_some() {
                        todo!("TRANSPILER ERROR: assert modifier is not allowed for comparison operations")
                    }

                    let InfixExpression::Comparison(comp) = e.borrow() else {
                        unreachable!()
                    };

                    let InfixOperation { lhs, operator, rhs } = comp;
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
                    .map(|a| match &a.expression.kind {
                        ExpressionKind::Closure(closure) => {
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

pub fn transpile_if(
    func: &FunctionCall,
    ctx: &Context<'_>,
    scope: &mut Scope<'_>,
    ty: Option<TypeElement>,
) -> String {
    debug_assert_eq!(func.identifier.as_str(), "if");
    assert_eq!(
        func.arguments.len(),
        2,
        "if should have two arguments: condition and body"
    );
    let condition = &func.arguments[0];
    let ExpressionKind::Closure(body) = &func.arguments[1].expression.kind else {
        todo!("TRANSPILER ERROR: second argument of if needs to be a body")
    };
    let condition = condition.transpile(ctx, scope);

    let mut body_scope = Scope::child(scope);
    body_scope.return_type = ty;
    format!(
        "if {condition} {{ {} }}",
        body.block.transpile(ctx, &mut body_scope)
    )
}

pub fn transpile_for(func: &FunctionCall, ctx: &Context<'_>, scope: &mut Scope<'_>) -> String {
    assert_eq!(
        func.arguments.len(),
        2,
        "TRANSPILE ERROR: for loop needs two arguments: an iterator and a closure"
    );
    let iterator = &func.arguments[0];
    let iter_ty = iterator.expression.infer_type(scope);
    let elem_ty = match &iter_ty {
        Some(ty) => match ty {
            TypeElement::Array(ty) => Some(&ty.elements),
            TypeElement::Dictionary(_ty) => todo!("for loop on dict"),
            TypeElement::OrderedDictionary(_ty) => todo!("for loop on ordered dict"),
            TypeElement::Set(ty) => Some(&ty.elements),
            TypeElement::Tuple(_ty) => todo!("TRANSPILE ERROR: Cannot iterate over tuple type"),
            TypeElement::Optional(_ty) => todo!("for loop on optional"),
            TypeElement::Result(_ty) => todo!("TRANSPILE ERROR: Cannot iterate over result type"),
            TypeElement::Plain(_ty) => todo!(),
            TypeElement::Generic(_ty) => todo!(),
            TypeElement::Never(_) => todo!(),
        },
        None => None,
    };
    let ExpressionKind::Closure(closure) = &func.arguments[1].expression.kind else {
        todo!("TRANSPILER ERROR: second argument of if needs to be a body")
    };
    let condition = iterator.transpile(ctx, scope);
    // TODO: auto-unfold tuples into multiple arguments
    assert!(
        closure.arguments.len() > 0,
        "TRANSPILER ERROR: for loop body at least one argument"
    );
    let element = if closure.arguments.len() == 1 {
        closure.arguments[0].ident.transpile(ctx, scope)
    } else {
        let elements = closure
            .arguments
            .iter()
            .map(|arg| arg.transpile(ctx, scope))
            .join(", ");
        format!("({elements})")
    };
    // HACK: just assume we need to revert the auto-inserted & for unknown types
    let condition = if iter_ty.is_none() {
        condition
            .strip_prefix("&(")
            .unwrap_or(&condition)
            .strip_suffix(")")
            .unwrap_or(&condition)
    } else {
        &condition
    };
    let mut prefix = "";
    if let Some(elem_ty) = elem_ty {
        if ctx.mapping.is_copy_type(&elem_ty) {
            prefix = "&"
        }
    }
    // TODO: try to figure out capacity and create vec with matching capacity
    // TODO: only do this when the body returns a value
    let mut body_scope = Scope::child(scope);
    let mut block: Vec<_> = closure
        .block
        .body
        .statements
        .iter()
        .map(|stmt| stmt.transpile(ctx, &mut body_scope))
        .collect();
    let len = block.len();
    // This allows for loops that automatically collect values produced in each iteration
    for (i, stmt) in block.iter_mut().enumerate() {
        if i == len - 1 {
            *stmt = format!("__result.push({stmt})")
        }
    }
    let block = block.join(";\n");
    format!(
        "{{
        let mut __result = Vec::new(); 
        for {prefix}{element} in {condition} {{ {block} }}
        __result
        }}"
    )
}

impl Transpile for FunctionCallArg {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        use DeclModifier as Mod;
        use ExpressionKind as Exp;
        let Self {
            modifier,
            expression,
        } = self;
        match (modifier, &expression.kind) {
            (Some(Mod::Let), _) => {
                todo!("TRANSPILER ERROR: Let modifier is not allowed for function call arguments")
            }
            (None, match_ident!(ident)) => {
                match scope
                    .get_variable(&ident)
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
            (Some(Mod::Mut), expr @ Exp::Infix(_) | expr @ match_ident!(_)) => {
                transpile!(ctx, scope, "&mut {}", expr)
            }
            (Some(Mod::Ref), expr @ Exp::Infix(_) | expr @ match_ident!(_)) => {
                transpile!(ctx, scope, "::std::sync::Arc::clone(&{})", expr)
            }
            _ => todo!("TRANSPILER ERROR: Modifier only allowed for fields or variables"),
        }
    }
}
