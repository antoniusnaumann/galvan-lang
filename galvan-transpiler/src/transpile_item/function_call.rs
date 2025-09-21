use std::borrow::Borrow;

use galvan_ast::TypeElement::{self};
use galvan_ast::{
    ComparisonOperator, DeclModifier, EnumConstructor, EnumConstructorArg, Expression,
    ExpressionKind, FunctionCall, FunctionCallArg, Ident, InfixExpression, InfixOperation,
    Ownership, TypeIdent,
};
use galvan_resolver::{Lookup, Scope, Variable};
use itertools::Itertools;

use crate::builtins::{CheckBuiltins, BORROWED_ITERATOR_FNS};
use crate::cast::{cast, transpile_unified, unify};
use crate::context::Context;
use crate::error::ErrorCollector;
use crate::macros::transpile;
use crate::transpile_item::closure::{transpile_closure, transpile_closure_argument};
use crate::transpile_item::statement::match_ident;
use crate::type_inference::InferType;
use crate::Transpile;

impl Transpile for FunctionCall {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        match self.identifier.as_str() {
            "panic" => format!(
                "panic!(\"{{}}\", {})",
                self.arguments.transpile(ctx, scope, errors)
            ),
            "println" => format!(
                "println!(\"{{}}\", {})",
                self.arguments.transpile(ctx, scope, errors)
            ),
            "print" => format!(
                "print!(\"{{}}\", {})",
                self.arguments.transpile(ctx, scope, errors)
            ),
            "debug" => format!(
                "println!(\"{{:?}}\", {})",
                self.arguments.transpile(ctx, scope, errors)
            ),
            "if" => {
                let ty = self.infer_type(scope, errors);
                match ty {
                    ty @ TypeElement::Optional(_) => {
                        let (cond, if_, if_ty) = transpile_if(self, ctx, scope, ty.clone(), errors);
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
                        let (cond, if_, _) = transpile_if(self, ctx, scope, ty, errors);
                        format!("{cond} {{ {if_} }}")
                    }
                }
            }
            "for" => transpile_for(self, ctx, scope, errors),
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
                        // TODO: Add proper error handling for invalid assert modifier
                        return format!("/* error: assert modifier not allowed */");
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
                    let lhs_type = lhs.infer_type(lhs_scope, errors);
                    let rhs_type = rhs.infer_type(rhs_scope, errors);
                    let has_number = lhs_type.is_number()
                        || rhs_type.is_number()
                        || lhs_type.is_infer()
                        || rhs_type.is_infer();
                    let (lhs, rhs) = match (
                        lhs.infer_owned(ctx, &lhs_scope, errors),
                        rhs.infer_owned(ctx, &rhs_scope, errors),
                    ) {
                        (
                            Ownership::SharedOwned | Ownership::UniqueOwned,
                            Ownership::Borrowed | Ownership::MutBorrowed,
                        ) if !has_number => {
                            let (lhs, rhs) =
                                transpile_unified(lhs, rhs, lhs_scope, rhs_scope, ctx, errors);
                            (format!("&({})", lhs), rhs)
                        }
                        (
                            Ownership::Borrowed | Ownership::MutBorrowed,
                            Ownership::SharedOwned | Ownership::UniqueOwned,
                        ) if !has_number => {
                            let (lhs, rhs) =
                                transpile_unified(lhs, rhs, lhs_scope, rhs_scope, ctx, errors);
                            (lhs, format!("&({})", rhs))
                        }
                        _ => {
                            // Special handling for __Number and Infer types in assertions
                            if has_number {
                                // For __Number types, we don't want to add & prefixes as they break type unification
                                let lhs_trans = lhs.transpile(ctx, lhs_scope, errors);
                                let rhs_trans = rhs.transpile(ctx, rhs_scope, errors);

                                // Apply unification to handle __Number -> wrapper type casting
                                let (unified_lhs, unified_rhs) =
                                    unify(&lhs_trans, &rhs_trans, &lhs_type, &rhs_type);

                                // Handle reference vs value mismatches - for any borrowed type with __Number/Infer, dereference
                                let lhs_ownership = lhs.infer_owned(ctx, &lhs_scope, errors);
                                let rhs_ownership = rhs.infer_owned(ctx, &rhs_scope, errors);

                                // Handle reference vs value mismatches based on actual ownership inference
                                let lhs_needs_deref = match (lhs_ownership, &lhs.kind) {
                                    (
                                        Ownership::Borrowed | Ownership::MutBorrowed,
                                        ExpressionKind::Ident(_),
                                    ) => true,
                                    _ => false,
                                };

                                let rhs_needs_deref = match (rhs_ownership, &rhs.kind) {
                                    (
                                        Ownership::Borrowed | Ownership::MutBorrowed,
                                        ExpressionKind::Ident(_),
                                    ) => true,
                                    _ => false,
                                };

                                let (final_lhs, final_rhs) =
                                    match (lhs_needs_deref, rhs_needs_deref) {
                                        (true, false) => {
                                            (format!("*{}", unified_lhs), unified_rhs.into_owned())
                                        }
                                        (false, true) => {
                                            (unified_lhs.into_owned(), format!("*{}", unified_rhs))
                                        }
                                        _ => (unified_lhs.into_owned(), unified_rhs.into_owned()),
                                    };
                                (final_lhs, final_rhs)
                            } else {
                                transpile_unified(lhs, rhs, lhs_scope, rhs_scope, ctx, errors)
                            }
                        }
                    };
                    match operator {
                        ComparisonOperator::Equal => {
                            format!(
                                "assert_eq!({lhs}, {rhs}, {})",
                                args.transpile(ctx, scope, errors)
                            )
                        }
                        ComparisonOperator::NotEqual => {
                            format!(
                                "assert_ne!({lhs}, {rhs}, {})",
                                args.transpile(ctx, scope, errors)
                            )
                        }
                        _ => format!("assert!({})", self.arguments.transpile(ctx, scope, errors)),
                    }
                }
                Some(_) => format!("assert!({})", self.arguments.transpile(ctx, scope, errors)),
                _ => {
                    // TODO: Add proper error handling for invalid assert arguments
                    format!("/* assert error */")
                }
            },
            s if BORROWED_ITERATOR_FNS.contains(&s) => {
                let ident = self.identifier.transpile(ctx, scope, errors);
                let args = self
                    .arguments
                    .iter()
                    .map(|a| {
                        let mut scope = Scope::child(scope)
                            .returns(TypeElement::infer(), Ownership::UniqueOwned);
                        match &a.expression.kind {
                            ExpressionKind::Closure(closure) => {
                                if a.modifier.is_some() {
                                    // TODO: Add proper error handling for invalid closure modifier
                                    return format!("/* error: closure modifier not allowed */");
                                }
                                transpile_closure(ctx, &mut scope, closure, true, errors)
                            }
                            _ => a.transpile(ctx, &mut scope, errors),
                        }
                    })
                    .join(", ");
                format!("{}({})", ident, args)
            }
            _ => transpile_fn_call(self, ctx, scope, errors),
        }
    }
}

fn transpile_fn_call(
    call: &FunctionCall,
    ctx: &Context<'_>,
    scope: &mut Scope,
    errors: &mut ErrorCollector,
) -> String {
    transpile_call_with_receiver(None, &call.identifier, &call.arguments, ctx, scope, errors)
}

pub fn transpile_call_with_receiver(
    receiver: Option<&Expression>,
    identifier: &Ident,
    arguments: &[FunctionCallArg],
    ctx: &Context<'_>,
    scope: &mut Scope,
    errors: &mut ErrorCollector,
) -> String {
    // Determine receiver type for method lookup
    let receiver_type = receiver.map(|r| r.infer_type(scope, errors));
        let receiver_ident = receiver_type.as_ref().and_then(|t| match t {
            TypeElement::Plain(basic) => Some(&basic.ident),
            TypeElement::Parametric(param) => Some(&param.base_type),
            _ => None,
        });
        
        // For Generic types, we need to create a temporary TypeIdent
        let temp_type_ident;
        let receiver_ident = if let Some(ident) = receiver_ident {
            Some(ident)
        } else if let Some(TypeElement::Generic(gen)) = receiver_type.as_ref() {
            temp_type_ident = TypeIdent::new(gen.ident.as_str());
            Some(&temp_type_ident)
        } else {
            None
        };    
    // Look up the function/method
    let func = ctx.lookup.resolve_function(receiver_ident, identifier, &[]);

    if let Some(func) = func {
        // Check if this is a generic function
        let generics = func.signature.collect_generics();
        let has_where_clause = func.signature.where_clause.is_some();
        let is_generic = !generics.is_empty() || has_where_clause;

        // Process arguments with proper casting
        let args = process_function_arguments(
            &func.signature.parameters.params,
            arguments,
            is_generic,
            ctx,
            scope,
            errors,
        );

        // Format the call based on whether it's a method or function
        if let Some(recv) = receiver {
            let receiver_transpiled = recv.transpile(ctx, scope, errors);
            format!("{}.{}({})", receiver_transpiled, identifier.transpile(ctx, scope, errors), args)
        } else {
            format!("{}({})", identifier.transpile(ctx, scope, errors), args)
        }
    } else {
        // Function not found - use fallback logic
        let args = arguments
            .iter()
            .map(|arg| {
                let mut arg_scope =
                    Scope::child(scope).returns(TypeElement::infer(), Ownership::Borrowed);
                arg.transpile(ctx, &mut arg_scope, errors)
            })
            .join(", ");

        if let Some(recv) = receiver {
            let receiver_transpiled = recv.transpile(ctx, scope, errors);
            if let Some(recv_type) = receiver_type {
                errors.warning(
                    format!("Function '{}' not found. Available functions: {}", 
                        identifier,
                        scope
                            .functions()
                            .iter()
                            .map(|f| f.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ), 
                    None
                );
            }
            format!("{}.{}({})", receiver_transpiled, identifier.transpile(ctx, scope, errors), args)
        } else {
            format!("{}({})", identifier.transpile(ctx, scope, errors), args)
        }
    }
}

fn process_function_arguments(
    params: &[galvan_ast::Param],
    arguments: &[FunctionCallArg],
    is_generic: bool,
    ctx: &Context<'_>,
    scope: &mut Scope,
    errors: &mut ErrorCollector,
) -> String {
    params
        .iter()
        .skip_while(|p| p.identifier.as_str() == "self")
        .zip(arguments)
        .map(|(param, arg)| {
            let ownership = match param.decl_modifier {
                Some(m) => match m {
                    DeclModifier::Let => {
                        errors.warning("Let modifier not yet implemented".to_string(), None);
                        Ownership::Borrowed
                    }
                    DeclModifier::Mut => Ownership::MutBorrowed,
                    DeclModifier::Ref => {
                        errors.warning("Ref modifier not yet implemented".to_string(), None);
                        Ownership::Borrowed
                    }
                },
                None => {
                    if ctx.mapping.is_copy(&param.param_type) {
                        Ownership::UniqueOwned
                    } else {
                        if is_generic {
                            // For generic functions, use conservative approach
                            Ownership::Borrowed
                        } else {
                            // For non-generic functions with known signature
                            Ownership::Borrowed
                        }
                    }
                }
            };
            let mut arg_scope = Scope::child(scope).returns(param.param_type.clone(), ownership);
            arg.transpile(ctx, &mut arg_scope, errors)
        })
        .join(", ")
}

pub fn transpile_if(
    func: &FunctionCall,
    ctx: &Context<'_>,
    scope: &mut Scope<'_>,
    ty: TypeElement,
    errors: &mut ErrorCollector,
) -> (String, String, TypeElement) {
    debug_assert_eq!(func.identifier.as_str(), "if");
    assert_eq!(
        func.arguments.len(),
        2,
        "if should have two arguments: condition and body"
    );
    let condition = &func.arguments[0];
    let ExpressionKind::Closure(body) = &func.arguments[1].expression.kind else {
        // TODO: Add proper error handling for invalid if body argument
        return (
            String::from("/* error */"),
            String::from("/* error */"),
            TypeElement::infer(),
        );
    };
    let mut condition_scope =
        Scope::child(scope).returns(TypeElement::bool(), Ownership::UniqueOwned);
    let condition = condition.transpile(ctx, &mut condition_scope, errors);

    let mut body_scope = Scope::child(scope).returns(ty, scope.ownership);
    let body_type = body.block.infer_type(&body_scope, errors);
    (
        format!("if {condition}"),
        format!("{}", body.block.transpile(ctx, &mut body_scope, errors)),
        body_type,
    )
}

fn transpile_for(
    func: &FunctionCall,
    ctx: &Context<'_>,
    scope: &mut Scope<'_>,
    errors: &mut ErrorCollector,
) -> String {
    if func.arguments.len() != 2 {
        // TODO: Add proper error handling for invalid for loop arguments
        return String::new();
    }
    let iterator = &func.arguments[0];
    let iter_ty = iterator.expression.infer_type(scope, errors);
    let elem_ty = match &iter_ty {
        TypeElement::Array(ty) => &ty.elements,
        TypeElement::Dictionary(_ty) => {
            errors.warning(
                "For loop on dictionary not yet implemented".to_string(),
                None,
            );
            &TypeElement::infer()
        }
        TypeElement::OrderedDictionary(_ty) => {
            errors.warning(
                "For loop on ordered dictionary not yet implemented".to_string(),
                None,
            );
            &TypeElement::infer()
        }
        TypeElement::Set(ty) => &ty.elements,
        TypeElement::Tuple(_ty) => {
            // TODO: Add proper error handling for tuple iteration
            &TypeElement::infer()
        }
        TypeElement::Optional(_ty) => {
            // TODO: Implement for loop on optional types
            &TypeElement::infer()
        }
        TypeElement::Result(_ty) => {
            // TODO: Add proper error handling for result iteration
            &TypeElement::infer()
        }
        TypeElement::Plain(_ty) => {
            errors.warning(
                "For loop on plain type not yet implemented".to_string(),
                None,
            );
            &TypeElement::infer()
        }
        TypeElement::Generic(_ty) => {
            errors.warning(
                "For loop on generic type not yet implemented".to_string(),
                None,
            );
            &TypeElement::infer()
        }
        TypeElement::Parametric(_ty) => {
            errors.warning(
                "For loop on parametric type not yet implemented".to_string(),
                None,
            );
            &TypeElement::infer()
        }
        TypeElement::Void(_) => &iter_ty,
        TypeElement::Infer(_) => &iter_ty,
        TypeElement::Never(_) => {
            errors.warning(
                "For loop on never type not yet implemented".to_string(),
                None,
            );
            &TypeElement::infer()
        }
    };
    let ExpressionKind::Closure(closure) = &func.arguments[1].expression.kind else {
        // TODO: Add proper error handling for invalid for body argument
        return String::new();
    };

    // Check iterator ownership - if exclusively owned (e.g., from function return),
    // use that ownership to avoid unnecessary borrowing
    let iter_ownership = iterator.expression.infer_owned(ctx, scope, errors);
    let scope_ownership = match iter_ownership {
        Ownership::UniqueOwned | Ownership::SharedOwned => iter_ownership,
        _ => Ownership::Borrowed,
    };

    let mut iter_scope = Scope::child(scope).returns(iter_ty.clone(), scope_ownership);
    let condition = iterator.transpile(ctx, &mut iter_scope, errors);

    fn get_iteration_type(parent: &TypeElement, errors: &mut ErrorCollector) -> TypeElement {
        match parent {
            TypeElement::Array(array) => array.elements.clone(),
            TypeElement::Dictionary(_) => {
                errors.warning(
                    "Collecting into dictionary not yet implemented".to_string(),
                    None,
                );
                TypeElement::infer()
            }
            TypeElement::OrderedDictionary(_) => {
                errors.warning(
                    "Collecting into ordered dictionary not yet implemented".to_string(),
                    None,
                );
                TypeElement::infer()
            }
            TypeElement::Set(_) => {
                errors.warning("Collecting into set not yet implemented".to_string(), None);
                TypeElement::infer()
            }
            TypeElement::Optional(opt) => get_iteration_type(&opt.inner, errors),
            TypeElement::Result(res) => get_iteration_type(&res.success, errors),
            TypeElement::Never(never) => TypeElement::Never(never.clone()),
            TypeElement::Infer(_) => TypeElement::infer(),
            TypeElement::Void(_) => TypeElement::void(),
            _ => {
                // TODO: Add proper error handling for invalid collect type
                TypeElement::infer()
            }
        }
    }

    // TODO: try to figure out capacity and create vec with matching capacity
    let iteration_return = get_iteration_type(&scope.return_type, errors);

    let mut body_scope = Scope::child(scope).returns(iteration_return, Ownership::UniqueOwned);
    let element = {
        if closure.parameters.is_empty() {
            // Implicit 'it' parameter case
            let it_ownership = if ctx.mapping.is_copy(elem_ty) {
                Ownership::UniqueOwned
            } else {
                Ownership::Borrowed
            };
            body_scope.declare_variable(Variable {
                ident: Ident::new("it"),
                modifier: DeclModifier::Let,
                ty: elem_ty.clone(),
                ownership: it_ownership,
            });
            "it".to_string()
        } else {
            // Existing explicit parameter handling
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
                        errors,
                    )
                })
                .join(", ");
            format!("({elements})")
        }
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
    // Check if this is a range expression - ranges iterate over owned values, not references
    let is_range_expression = match &iterator.expression.kind {
        ExpressionKind::Infix(infix) => matches!(**infix, InfixExpression::Range(_)),
        _ => false,
    };

    let prefix = if is_range_expression {
        // Ranges iterate over owned values directly
        ""
    } else if ctx.mapping.is_copy(&elem_ty) {
        // Arrays/vectors iterate over references that need to be destructured
        "&"
    } else {
        ""
    };

    let mut block: Vec<_> = closure
        .block
        .body
        .statements
        .iter()
        .map(|stmt| stmt.transpile(ctx, &mut body_scope, errors))
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
            body_scope.return_type.transpile(ctx, scope, errors)
        )
    }
}

impl Transpile for [FunctionCallArg] {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        self.iter()
            .map(|arg| {
                // TODO: use type informations here somehow
                let mut scope =
                    Scope::child(scope).returns(TypeElement::infer(), Ownership::default());
                arg.transpile(ctx, &mut scope, errors)
            })
            .join(", ")
    }
}

impl Transpile for FunctionCallArg {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        use DeclModifier as Mod;
        use ExpressionKind as Exp;
        let Self {
            modifier,
            expression,
        } = self;
        // TODO: typecheck expression and expected type

        match (modifier, &expression.kind) {
            (Some(Mod::Let), _) => {
                // TODO: Add proper error handling for invalid let modifier
                String::new()
            }
            (None, match_ident!(ident)) => {
                let variable_ownership = scope
                    .get_variable(&ident)
                    .unwrap_or_else(|| {
                        panic!(
                            "TODO: ERROR: undeclared variable {ident}, scope: {:#?}",
                            scope
                        )
                    })
                    .ownership;

                match (variable_ownership, scope.ownership) {
                    // If function expects a reference but variable is owned, borrow it
                    (Ownership::UniqueOwned | Ownership::SharedOwned, Ownership::Borrowed) => {
                        transpile!(ctx, scope, errors, "&{}", ident)
                    }
                    // If function expects a mutable reference but variable is owned, mut borrow it
                    (Ownership::UniqueOwned | Ownership::SharedOwned, Ownership::MutBorrowed) => {
                        transpile!(ctx, scope, errors, "&mut {}", ident)
                    }
                    // Standard cases based on variable ownership
                    (Ownership::SharedOwned, _) => {
                        transpile!(ctx, scope, errors, "&{}", ident)
                    }
                    (Ownership::Borrowed | Ownership::MutBorrowed | Ownership::UniqueOwned, _) => {
                        transpile!(ctx, scope, errors, "{}", ident)
                    }
                    (Ownership::Ref, _) => {
                        transpile!(ctx, scope, errors, "{}.lock().unwrap()", ident)
                    }
                }
            }
            (None, Exp::Closure(closure)) => {
                transpile!(ctx, scope, errors, "{}", closure)
            }
            (None, _expr) => {
                // For function arguments, we need to handle different scenarios:
                // 1. Known function with concrete type expectations -> use cast system
                // 2. Unknown function but with ownership info -> apply ownership selectively
                // 3. No type or ownership info -> just transpile directly
                let expected_type = scope.return_type.clone();

                if !expected_type.is_infer() {
                    // We have concrete type expectations, apply full casting logic
                    let is_copy = ctx.mapping.is_copy(&expected_type);
                    let ownership = if is_copy {
                        Ownership::UniqueOwned
                    } else {
                        scope.ownership
                    };
                    let converted_expression =
                        cast(expression, &expected_type, ownership, ctx, scope, errors);

                    transpile!(ctx, scope, errors, "{}", converted_expression)
                } else {
                    // Expected type is inferred, but we may still need to apply ownership
                    let expr_result = transpile!(ctx, scope, errors, "{}", expression);

                    // Only apply borrowing for expressions that produce owned values
                    // that need to be borrowed for function calls
                    let should_borrow = match (&scope.ownership, &expression.kind) {
                        (Ownership::Borrowed, galvan_ast::ExpressionKind::FunctionCall(_)) => true,
                        (
                            Ownership::Borrowed,
                            galvan_ast::ExpressionKind::Literal(
                                galvan_ast::Literal::StringLiteral(_),
                            ),
                        ) => true,
                        (Ownership::Borrowed, galvan_ast::ExpressionKind::Infix(_)) => false, // arithmetic, member access, etc.
                        (Ownership::Borrowed, galvan_ast::ExpressionKind::Ident(_)) => false, // handled separately above
                        (Ownership::MutBorrowed, galvan_ast::ExpressionKind::FunctionCall(_)) => {
                            true
                        }
                        (
                            Ownership::MutBorrowed,
                            galvan_ast::ExpressionKind::Literal(
                                galvan_ast::Literal::StringLiteral(_),
                            ),
                        ) => true,
                        _ => false,
                    };

                    if should_borrow {
                        match scope.ownership {
                            Ownership::Borrowed => format!("&({})", expr_result),
                            Ownership::MutBorrowed => format!("&mut ({})", expr_result),
                            _ => expr_result,
                        }
                    } else {
                        expr_result
                    }
                }
            }
            // TODO: Check if the infix expression is a member field access
            (Some(Mod::Mut), expr @ Exp::Infix(_) | expr @ match_ident!(_)) => {
                transpile!(ctx, scope, errors, "&mut {}", expr)
            }
            (Some(Mod::Ref), expr @ Exp::Infix(_) | expr @ match_ident!(_)) => {
                transpile!(ctx, scope, errors, "::std::sync::Arc::clone(&{})", expr)
            }
            _ => {
                // TODO: Add proper error handling for invalid modifier usage
                String::new()
            }
        }
    }
}

impl Transpile for EnumConstructor {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let enum_access = self.enum_access.transpile(ctx, scope, errors);

        if self.arguments.is_empty() {
            // Simple enum variant access: Color::Transparent
            enum_access
        } else if self.arguments.iter().all(|arg| arg.field_name.is_none()) {
            // All anonymous arguments: Color::Gray(128)
            let args = self
                .arguments
                .iter()
                .map(|arg| arg.expression.transpile(ctx, scope, errors))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({})", enum_access, args)
        } else {
            // Named arguments: Color::Rgb { r: 100, g: 10, b: 150 }
            let args = self
                .arguments
                .iter()
                .map(|arg| arg.transpile(ctx, scope, errors))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} {{ {} }}", enum_access, args)
        }
    }
}

impl Transpile for EnumConstructorArg {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let expr = self.expression.transpile(ctx, scope, errors);

        if let Some(ref field_name) = self.field_name {
            format!("{}: {}", field_name.as_str(), expr)
        } else {
            // Anonymous argument
            if let Some(ref modifier) = self.modifier {
                match modifier {
                    DeclModifier::Mut => format!("&mut {}", expr),
                    DeclModifier::Ref => format!("::std::sync::Arc::clone(&{})", expr),
                    DeclModifier::Let => expr,
                }
            } else {
                expr
            }
        }
    }
}
