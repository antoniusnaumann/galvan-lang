use crate::builtins::{CheckBuiltins, BORROWED_ITERATOR_FNS};
use crate::cast::{cast, transpile_unified, unify};
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
                    ty @ TypeElement::Optional(_) => {
                        let (cond, if_, if_ty) = transpile_if(self, ctx, scope, ty.clone());
                        // For __Number types, we need to wrap them in Some()
                        let cast_if = if if_ty.is_number() {
                            format!("Some({if_})")
                        } else {
                            let (if_, _) = unify(&if_, "", &if_ty, &ty);
                            if_.into_owned()
                        };
                        format!("{cond} {{ {cast_if} }} else {{ None }}")
                    }
                    ty => {
                        let (cond, if_, _) = transpile_if(self, ctx, scope, ty);
                        format!("{cond} {{ {if_} }}")
                    }
                }
            }
            "for" => transpile_for(self, ctx, scope),
            "assert" => match self.arguments.first() {
                Some(FunctionCallArg {
                    modifier,
                    expression:
                        Expression {
                            kind: ExpressionKind::Infix(e),
                            span: _,
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

                    let mut lhs_scope = Scope::child(scope);
                    let mut rhs_scope = Scope::child(scope);
                    let lhs_scope = &mut lhs_scope;
                    let rhs_scope = &mut rhs_scope;
                    
                     // Check if either side is a __Number for special handling
                     let lhs_type = lhs.infer_type(lhs_scope);
                     let rhs_type = rhs.infer_type(rhs_scope);
                     let has_number = lhs_type.is_number() || rhs_type.is_number() || lhs_type.is_infer() || rhs_type.is_infer();                    
                     let (lhs, rhs) = match (
                        lhs.infer_owned(ctx, &lhs_scope),
                        rhs.infer_owned(ctx, &rhs_scope),
                    ) {
                        (
                            Ownership::SharedOwned | Ownership::UniqueOwned,
                            Ownership::Borrowed | Ownership::MutBorrowed,
                        ) if !has_number => {
                            let (lhs, rhs) = transpile_unified(lhs, rhs, lhs_scope, rhs_scope, ctx);
                            (format!("&({})", lhs), rhs)
                        }
                        (
                            Ownership::Borrowed | Ownership::MutBorrowed,
                            Ownership::SharedOwned | Ownership::UniqueOwned,
                        ) if !has_number => {
                            let (lhs, rhs) = transpile_unified(lhs, rhs, lhs_scope, rhs_scope, ctx);
                            (lhs, format!("&({})", rhs))
                        }
                        _ => {
                            // Special handling for __Number and Infer types in assertions
                            if has_number {
                                // For __Number types, we don't want to add & prefixes as they break type unification
                                let lhs_trans = lhs.transpile(ctx, lhs_scope);
                                let rhs_trans = rhs.transpile(ctx, rhs_scope);
                                
                                // Apply unification to handle __Number -> wrapper type casting
                                let (unified_lhs, unified_rhs) = unify(&lhs_trans, &rhs_trans, &lhs_type, &rhs_type);
                                
                                // Handle reference vs value mismatches - for any borrowed type with __Number/Infer, dereference
                                let lhs_ownership = lhs.infer_owned(ctx, &lhs_scope);
                                let rhs_ownership = rhs.infer_owned(ctx, &rhs_scope);
                                
                                // Special case: pattern match variables like 'ok', 'err' are often borrowed but
                                // not correctly detected by ownership inference
                                let lhs_needs_deref = match lhs_ownership {
                                    Ownership::Borrowed | Ownership::MutBorrowed => true,
                                    // Special case for common pattern match variable names that are usually &T
                                    _ if (lhs_trans == "ok" || lhs_trans == "err" || lhs_trans == "i") 
                                         && rhs_type.is_number() => true,
                                    _ => false
                                };
                                
                                let rhs_needs_deref = match rhs_ownership {
                                    Ownership::Borrowed | Ownership::MutBorrowed => true,
                                    // Special case for common pattern match variable names that are usually &T  
                                    _ if (rhs_trans == "ok" || rhs_trans == "err" || rhs_trans == "i")
                                         && lhs_type.is_number() => true,
                                    _ => false
                                };
                                
                                let (final_lhs, final_rhs) = match (lhs_needs_deref, rhs_needs_deref) {
                                    (true, false) => (format!("*{}", unified_lhs), unified_rhs.into_owned()),
                                    (false, true) => (unified_lhs.into_owned(), format!("*{}", unified_rhs)),
                                    _ => (unified_lhs.into_owned(), unified_rhs.into_owned())
                                };
                                (final_lhs, final_rhs)
                            } else {
                                transpile_unified(lhs, rhs, lhs_scope, rhs_scope, ctx)
                            }
                        }
                    };
                    match operator {
                        ComparisonOperator::Equal => {
                            format!("assert_eq!({lhs}, {rhs}, {})", args.transpile(ctx, scope))
                        }
                        ComparisonOperator::NotEqual => {
                            format!("assert_ne!({lhs}, {rhs}, {})", args.transpile(ctx, scope))
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
                        let mut scope = Scope::child(scope)
                            .returns(TypeElement::infer(), Ownership::UniqueOwned);
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
                let ownership = match param.decl_modifier {
                    Some(m) => match m {
                        DeclModifier::Let => todo!(),
                        DeclModifier::Mut => Ownership::MutBorrowed,
                        DeclModifier::Ref => todo!(),
                    },
                    None => Ownership::Borrowed,
                };
                let mut arg_scope =
                    Scope::child(scope).returns(param.param_type.clone(), ownership);
                arg.transpile(ctx, &mut arg_scope)
            })
            .join(", ");

        format!("{}({})", identifier.transpile(ctx, scope), args)
    } else {
        let args = arguments
            .iter()
            .map(|arg| {
                let mut arg_scope =
                    Scope::child(scope).returns(TypeElement::infer(), Ownership::Borrowed);
                arg.transpile(ctx, &mut arg_scope)
            })
            .join(", ");

        format!("{}({})", identifier.transpile(ctx, scope), args)
    }
}

pub fn transpile_if(
    func: &FunctionCall,
    ctx: &Context<'_>,
    scope: &mut Scope<'_>,
    ty: TypeElement,
) -> (String, String, TypeElement) {
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
    let mut condition_scope =
        Scope::child(scope).returns(TypeElement::bool(), Ownership::UniqueOwned);
    let condition = condition.transpile(ctx, &mut condition_scope);

    let mut body_scope = Scope::child(scope).returns(ty, scope.ownership);
    (
        format!("if {condition}"),
        format!("{}", body.block.transpile(ctx, &mut body_scope)),
        body.block.infer_type(&body_scope),
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
        TypeElement::Array(ty) => &ty.elements,
        TypeElement::Dictionary(_ty) => todo!("for loop on dict"),
        TypeElement::OrderedDictionary(_ty) => todo!("for loop on ordered dict"),
        TypeElement::Set(ty) => &ty.elements,
        TypeElement::Tuple(_ty) => todo!("TRANSPILE ERROR: Cannot iterate over tuple type"),
        TypeElement::Optional(_ty) => todo!("for loop on optional"),
        TypeElement::Result(_ty) => todo!("TRANSPILE ERROR: Cannot iterate over result type"),
        TypeElement::Plain(_ty) => todo!(),
        TypeElement::Generic(_ty) => todo!(),
        TypeElement::Void(_) => &iter_ty,
        TypeElement::Infer(_) => &iter_ty,
        TypeElement::Never(_) => todo!(),
    };
    let ExpressionKind::Closure(closure) = &func.arguments[1].expression.kind else {
        todo!("TRANSPILER ERROR: second argument of if needs to be a body")
    };
    
    // Check iterator ownership - if exclusively owned (e.g., from function return), 
    // use that ownership to avoid unnecessary borrowing
    let iter_ownership = iterator.expression.infer_owned(ctx, scope);
    let scope_ownership = match iter_ownership {
        Ownership::UniqueOwned | Ownership::SharedOwned => iter_ownership,
        _ => Ownership::Borrowed,
    };
    
    let mut iter_scope = Scope::child(scope).returns(iter_ty.clone(), scope_ownership);
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
            TypeElement::Infer(_) => TypeElement::infer(),
            TypeElement::Void(_) => TypeElement::void(),
            _ => todo!("TRANSPILER ERROR: can only collect for loops into vec"),
        }
    }

    // TODO: try to figure out capacity and create vec with matching capacity
    let iteration_return = get_iteration_type(&scope.return_type);

    let mut body_scope = Scope::child(scope).returns(iteration_return, Ownership::UniqueOwned);
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
                    if ctx.mapping.is_copy(elem_ty) {
                        Ownership::UniqueOwned
                    } else {
                        Ownership::Borrowed
                    },
                    true,
                )
            })
            .join(", ");
        format!("({elements})")
    };
    // HACK: just assume we need to revert the auto-inserted & for unknown types
    let condition = if iter_ty.is_infer() {
        condition
            .strip_prefix("&(")
            .and_then(|s| s.strip_suffix(")"))
            .unwrap_or(&condition)
    } else {
        &condition
    };
    let prefix = if ctx.mapping.is_copy(&elem_ty) {
        "&"
    } else {
        ""
    };

    let mut block: Vec<_> = closure
        .block
        .body
        .statements
        .iter()
        .map(|stmt| stmt.transpile(ctx, &mut body_scope))
        .collect();

    if !scope.return_type.is_void() {
        let len = block.len();
        // This allows for loops that automatically collect values produced in each iteration
        for (i, stmt) in block.iter_mut().enumerate() {
            if i == len - 1 {
                *stmt = format!("__result.push({stmt})")
            }
        }
    }
    let block = block.join(";\n");

    if scope.return_type.is_void() {
        format!("for {prefix}{element} in {condition} {{ {block}; }}")
    } else {
        format!(
            "{{
                let mut __result: ::std::vec::Vec<{}> = ::std::vec::Vec::new(); 
                for {prefix}{element} in {condition} {{ {block} }}
                __result
            }}",
            body_scope.return_type.transpile(ctx, scope)
        )
    }
}

impl Transpile for [FunctionCallArg] {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        self.iter()
            .map(|arg| {
                // TODO: use type informations here somehow
                let mut scope =
                    Scope::child(scope).returns(TypeElement::infer(), Ownership::default());
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
                    Ownership::SharedOwned => {
                        transpile!(ctx, scope, "&{}", ident)
                    }
                    Ownership::Borrowed | Ownership::MutBorrowed | Ownership::UniqueOwned => {
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
            (None, expr) => {
                let ref return_type = scope.return_type;
                let return_type = if return_type.is_infer() {
                    expr.infer_type(scope)
                } else {
                    return_type.clone()
                };
                let is_copy = ctx.mapping.is_copy(&return_type);
                let expression = cast(
                    expression,
                    &return_type,
                    if is_copy {
                        Ownership::UniqueOwned
                    } else {
                        // Use the scope's return ownership instead of always forcing Borrowed
                        scope.ownership
                    },
                    ctx,
                    scope,
                );

                transpile!(ctx, scope, "{}", expression)
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
