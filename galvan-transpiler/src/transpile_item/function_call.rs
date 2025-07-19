use crate::builtins::{CheckBuiltins, BORROWED_ITERATOR_FNS};
use crate::cast::cast;
use crate::context::Context;
use crate::macros::transpile;
use crate::transpile_item::closure::{transpile_closure, transpile_closure_argument};
use crate::transpile_item::statement::match_ident;
use crate::type_inference::InferType;
use crate::Transpile;
use galvan_ast::TypeElement::{self};
use galvan_ast::{
    ComparisonOperator, DeclModifier, Expression, ExpressionKind, FunctionCall, FunctionCallArg,
    InfixExpression, InfixOperation, Ownership,
};
use galvan_resolver::{Lookup, Scope};
use itertools::Itertools;
use std::borrow::{Borrow, Cow};

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

                    let (lhs, rhs) =
                        match (lhs.infer_owned(ctx, scope), rhs.infer_owned(ctx, scope)) {
                            (
                                Ownership::Owned | Ownership::Copy,
                                Ownership::Borrowed | Ownership::MutBorrowed,
                            ) => (
                                transpile!(ctx, scope, "&({})", lhs),
                                rhs.transpile(ctx, scope),
                            ),
                            (
                                Ownership::Borrowed | Ownership::MutBorrowed,
                                Ownership::Owned | Ownership::Copy,
                            ) => (
                                lhs.transpile(ctx, scope),
                                transpile!(ctx, scope, "&({})", rhs),
                            ),
                            _ => (lhs.transpile(ctx, scope), rhs.transpile(ctx, scope)),
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
                    .map(|a| {
                        let mut scope = Scope::child(scope).returns(Some(TypeElement::infer()));
                        match &a.expression.kind {
                            ExpressionKind::Closure(closure) => {
                                assert!(
                            a.modifier.is_none(),
                            "TRANSPILER ERROR: closure modifier not allowed for iterator functions"
                        );
                                transpile_closure(ctx, &mut scope, closure, true)
                            }
                            _ => a.transpile(ctx, &mut scope),
                        }
                    })
                    .join(", ");
                format!("{}({})", ident, args)
            }
            _ => transpile_fn_call(self, ctx, scope),
        }
    }
}

fn transpile_fn_call(call: &FunctionCall, ctx: &Context<'_>, scope: &mut Scope) -> String {
    let func = ctx.lookup.resolve_function(None, &call.identifier, &[]);
    let FunctionCall {
        identifier,
        arguments,
    } = call;

    if let Some(func) = func {
        let args = &func
            .signature
            .parameters
            .params
            .iter()
            .skip_while(|p| p.identifier.as_str() == "self")
            .zip(arguments)
            .map(|(param, arg)| {
                let mut arg_scope = Scope::child(scope).returns(Some(param.param_type.clone()));
                arg.transpile(ctx, &mut arg_scope)
            })
            .join(", ");

        format!("{}({})", identifier, args)
    } else {
        let args = arguments
            .iter()
            .map(|arg| {
                let mut arg_scope = Scope::child(scope).returns(Some(TypeElement::infer()));
                arg.transpile(ctx, &mut arg_scope)
            })
            .join(", ");

        format!("{}({})", identifier, args)
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
    let mut condition_scope = Scope::child(scope).returns(Some(TypeElement::bool()));
    let condition = condition.transpile(ctx, &mut condition_scope);

    let mut body_scope = Scope::child(scope).returns(ty);
    format!(
        "if {condition} {{ {} }}",
        body.block.transpile(ctx, &mut body_scope)
    )
}

fn transpile_for(func: &FunctionCall, ctx: &Context<'_>, scope: &mut Scope<'_>) -> String {
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
    let mut iter_scope = Scope::child(scope).returns(Some(
        iter_ty.clone().unwrap_or_else(|| TypeElement::infer()),
    ));
    let condition = iterator.transpile(ctx, &mut iter_scope);
    // TODO: auto-unfold tuples into multiple arguments
    assert!(
        closure.parameters.len() > 0,
        "TRANSPILER ERROR: for loop body at least one argument"
    );

    fn get_iteration_type(parent: &TypeElement) -> TypeElement {
        match parent {
            TypeElement::Array(array) => array.elements.clone(),
            TypeElement::Dictionary(_) => todo!("allow collecting into dict"),
            TypeElement::OrderedDictionary(_) => todo!("allow collecting into ordered dict"),
            TypeElement::Set(_) => todo!("allow collecting into set"),
            TypeElement::Optional(opt) => get_iteration_type(&opt.inner),
            TypeElement::Result(res) => get_iteration_type(&res.success),
            TypeElement::Never(never) => TypeElement::Never(never.clone()),
            TypeElement::Plain(_) if parent.is_infer() => TypeElement::infer(),
            _ => todo!("TRANSPILER ERROR: can only collect for loops into vec"),
        }
    }

    // TODO: try to figure out capacity and create vec with matching capacity
    let iteration_return = if let Some(ref ret) = scope.return_type {
        Some(get_iteration_type(ret))
    } else {
        None
    };

    let mut body_scope = Scope::child(scope).returns(iteration_return);
    let element = {
        let elements = closure
            .parameters
            .iter()
            .map(|arg| {
                transpile_closure_argument(
                    ctx,
                    &mut body_scope,
                    arg,
                    false,
                    if elem_ty.is_some_and(|ty| ctx.mapping.is_copy(ty)) {
                        Ownership::Copy
                    } else {
                        Ownership::Borrowed
                    },
                )
            })
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
        if ctx.mapping.is_copy(&elem_ty) {
            prefix = "&"
        }
    }
    let mut block: Vec<_> = closure
        .block
        .body
        .statements
        .iter()
        .map(|stmt| stmt.transpile(ctx, &mut body_scope))
        .collect();

    if scope.return_type.is_some() {
        let len = block.len();
        // This allows for loops that automatically collect values produced in each iteration
        for (i, stmt) in block.iter_mut().enumerate() {
            if i == len - 1 {
                *stmt = format!("__result.push({stmt})")
            }
        }
    }
    let block = block.join(";\n");

    if scope.return_type.is_none() {
        format!("for {prefix}{element} in {condition} {{ {block} }}")
    } else {
        format!(
            "{{
        let mut __result = Vec::new(); 
        for {prefix}{element} in {condition} {{ {block} }}
        __result
        }}"
        )
    }
}

impl Transpile for [FunctionCallArg] {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        self.iter()
            .map(|arg| {
                // TODO: use type informations here somehow
                let mut scope = Scope::child(scope).returns(Some(TypeElement::infer()));
                arg.transpile(ctx, &mut scope)
            })
            .join(", ")
    }
}

impl Transpile for FunctionCallArg {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        use DeclModifier as Mod;
        use ExpressionKind as Exp;
        let Self {
            modifier,
            expression,
        } = self;
        // TODO: typecheck expression and expected type

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
                let return_type = scope
                    .return_type
                    .as_ref()
                    .expect("function arguments must have a scope return type");
                let return_type = if return_type.is_infer() {
                    if let Some(ty) = expression.infer_type(scope) {
                        Cow::Owned(ty)
                    } else {
                        Cow::Borrowed(return_type)
                    }
                } else {
                    Cow::Borrowed(return_type)
                };
                let is_copy = ctx.mapping.is_copy(&return_type);

                if is_copy {
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
