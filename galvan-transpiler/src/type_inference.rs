use galvan_ast::{
    ArithmeticOperator, ArrayLiteral, ArrayTypeItem, BasicTypeItem, Block, Body, CollectionLiteral,
    CollectionOperator, DictLiteral, DictLiteralElement, DictionaryTypeItem, ElseExpression,
    Expression, ExpressionKind, FunctionCall, Group, InfixExpression, InfixOperation, Literal,
    MemberOperator, NeverTypeItem, OptionalTypeItem, OrderedDictLiteral, OrderedDictionaryTypeItem,
    Ownership, PostfixExpression, SetLiteral, SetTypeItem, Span, Statement, TypeDecl, TypeElement,
    TypeIdent, UnwrapOperator,
};
use galvan_resolver::{Lookup, Scope};
use itertools::Itertools;

use crate::{
    builtins::{CheckBuiltins, IsSame},
    context::Context,
    error::{ErrorCollector, TranspilerError},
};

pub(crate) trait InferType {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement;

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership;
}


/// Safe helper for type mismatches in ElseExpression
fn handle_else_type_mismatch(
    receiver_type: &TypeElement, 
    block_type: &TypeElement,
    errors: &mut ErrorCollector
) -> TypeElement {
    errors.error(TranspilerError::TypeMismatch {
        expected: "matching types for if and else expressions".to_string(),
        found: format!("if: {:?}, else: {:?}", receiver_type, block_type),
    });
    TypeElement::infer()
}

/// Safe helper for unknown identifiers
fn handle_unknown_identifier(
    ident: &str,
    scope: &Scope,
    errors: &mut ErrorCollector
) -> TypeElement {
    // Get available variable names for suggestions
    let available_vars = scope.variables
        .keys()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    
    errors.suggest_similar_identifier(ident, &available_vars, None);
    TypeElement::infer()
}

impl InferType for ElseExpression {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        let receiver_type = self.receiver.infer_type(scope, errors);
        let block_type = self.block.infer_type(scope, errors);

        match (receiver_type, block_type) {
            (TypeElement::Optional(receiver), TypeElement::Never(_)) => receiver.inner,
            (ty, TypeElement::Never(_)) | (TypeElement::Never(_), ty) => ty,
            (receiver_type, block_type) if receiver_type.is_same(&block_type) => receiver_type,
            (receiver_type, TypeElement::Infer(_)) => receiver_type,
            (TypeElement::Optional(receiver_type), block_type) => {
                if receiver_type.inner.is_same(&block_type) {
                    block_type
                } else {
                    // Return an inferred type and log a warning for now
                    // TODO: This will be supported when type unions are implemented
                    errors.warning("Types of if and else expression don't match, union types not yet supported".to_string(), None);
                    TypeElement::infer()
                }
            }
            (TypeElement::Infer(_), block_type) => block_type,
            (ref receiver_type, ref block_type) => {
                handle_else_type_mismatch(receiver_type, block_type, errors)
            }
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        self.block.infer_owned(ctx, scope)
    }
}

impl InferType for Block {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        // Create a child scope for this block to handle variable scoping properly
        // For type inference, we don't need to mutate the scope, just create a child context
        let block_scope = Scope::child(scope);
        self.body.infer_type(&block_scope, errors)
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        // Create a child scope for this block to handle variable scoping properly
        let block_scope = Scope::child(scope);
        self.body.infer_owned(ctx, &block_scope)
    }
}

impl InferType for Body {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        self.statements
            .last()
            .map_or_else(|| TypeElement::void(), |stmt| stmt.infer_type(scope, errors))
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        self.statements
            .last()
            .map(|stmt| stmt.infer_owned(ctx, scope))
            .unwrap_or_default()
    }
}

impl InferType for Statement {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        match self {
            Statement::Assignment(_) => TypeElement::void(),
            Statement::Expression(expr) => expr.infer_type(scope, errors),
            Statement::Declaration(_) => TypeElement::void(),
            Statement::Return(ret) => {
                if ret.is_explicit {
                    TypeElement::Never(NeverTypeItem { span: ret.span })
                } else {
                    ret.expression.infer_type(scope, errors)
                }
            }
            Statement::Throw(throw) => TypeElement::Never(NeverTypeItem { span: throw.span }), // Statement::Block(block) => block.infer_type(scope),
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match self {
            Statement::Expression(expr) => expr.infer_owned(ctx, scope),
            _ => Ownership::SharedOwned,
        }
    }
}

impl InferType for Expression {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        self.kind.infer_type(scope, errors)
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        self.kind.infer_owned(ctx, scope)
    }
}

impl InferType for ExpressionKind {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        match self {
            ExpressionKind::Closure(_) => {
                // TODO: Implement type inference for closure
                TypeElement::infer()
            }
            ExpressionKind::ElseExpression(e) => e.infer_type(scope, errors),
            ExpressionKind::CollectionLiteral(collection) => collection.infer_type(scope, errors),
            ExpressionKind::FunctionCall(call) => call.infer_type(scope, errors),
            ExpressionKind::ConstructorCall(constructor) => BasicTypeItem {
                ident: TypeIdent::new(constructor.identifier.clone()),
                span: Span::default(),
            }
            .into(),
            ExpressionKind::EnumAccess(access) => TypeElement::Plain(BasicTypeItem {
                ident: access.target.clone(),
                span: Span::default(),
            }),
            ExpressionKind::Literal(literal) => literal.infer_type(scope, errors),
            ExpressionKind::Ident(ident) => {
                let Some(var) = scope.get_variable(ident) else {
                    handle_unknown_identifier(ident.as_str(), scope, errors);
                    return TypeElement::infer();
                };
                // println!("cargo::warning=got {:?} named {}", ty, ident);
                var.ty.clone()
            }
            ExpressionKind::Postfix(postfix) => postfix.infer_type(scope, errors),
            ExpressionKind::Infix(operation) => operation.infer_type(scope, errors),
            ExpressionKind::Group(Group { inner }) => inner.infer_type(scope, errors),
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match self {
            ExpressionKind::ElseExpression(e) => {
                let if_owned = e.receiver.infer_owned(ctx, scope);
                let else_owned = e.block.infer_owned(ctx, scope);

                match (if_owned, else_owned) {
                    // TODO: is this correct?
                    (a, b) if a == b => a,
                    _ => Ownership::Borrowed,
                }
            }
            ExpressionKind::FunctionCall(call) => call.infer_owned(ctx, scope),
            ExpressionKind::Infix(infix) => infix.infer_owned(ctx, scope),
            ExpressionKind::Postfix(postfix) => postfix.infer_owned(ctx, scope),
            ExpressionKind::CollectionLiteral(_) => Ownership::UniqueOwned,
            // TODO: this might be copy
            ExpressionKind::ConstructorCall(_) => Ownership::UniqueOwned,
            // TODO: this might be copy
            ExpressionKind::EnumAccess(_) => Ownership::SharedOwned,
            ExpressionKind::Literal(literal) => literal.infer_owned(ctx, scope),
            ExpressionKind::Ident(ident) => {
                let var = scope.get_variable(ident);
                if let Some(var) = var {
                    var.ownership
                } else {
                    // Variable not found in scope, use borrowed as fallback
                    Ownership::Borrowed
                }
            }
            ExpressionKind::Closure(_closure) => {
                eprintln!("TRANSPILER WARNING: Ownership inference for closures not yet implemented");
                Ownership::Borrowed
            }
            ExpressionKind::Group(_group) => {
                eprintln!("TRANSPILER WARNING: Ownership inference for groups not yet implemented");
                Ownership::Borrowed
            }
        }
    }
}

impl InferType for FunctionCall {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        if self.identifier.as_str() == "if" {
            self.arguments
                .last()
                .map(|last| {
                    let ExpressionKind::Closure(ref closure) = last.expression.kind else {
                        panic!("'if' is missing body")
                    };
                    let ty = closure
                        .block
                        .body
                        .statements
                        .last()
                        .map(|stmt| stmt.infer_type(scope, errors))
                        .unwrap_or_default();

                    match ty {
                        TypeElement::Never(_) | TypeElement::Void(_) => ty,
                        ty => TypeElement::Optional(
                            OptionalTypeItem {
                                inner: ty,
                                span: Span::default(),
                            }
                            .into(),
                        ),
                    }
                })
                .unwrap_or_default()
        } else if self.identifier.as_str() == "for" {
            // TODO: infer type of last statement
            Box::new(ArrayTypeItem {
                elements: TypeElement::infer(),
                span: Span::default(),
            })
            .into()
        } else {
            let func = scope.resolve_function(None, &self.identifier, &[]);

            if let Some(func) = func {
                func.signature.return_type.clone()
            } else {
                TypeElement::infer()
            }
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        if self.identifier.as_str() == "if" {
            self.arguments
                .last()
                .map(|last| {
                    let ExpressionKind::Closure(ref closure) = last.expression.kind else {
                        panic!("'if' is missing body")
                    };
                    closure
                        .block
                        .body
                        .statements
                        .last()
                        .map(|stmt| stmt.infer_owned(ctx, scope))
                        .unwrap_or_default()
                })
                .unwrap_or_default()
        } else {
            let func = scope.resolve_function(None, &self.identifier, &[]);

            if let Some(_func) = func {
                // TODO: lookup function, some Rust functions return borrowed values
                Ownership::UniqueOwned
            } else {
                // In galvan, function return values are always owned and since they would be dropped if not consumed, they are considered "unique", that is, never require cloning
                Ownership::UniqueOwned
            }
        }
    }
}

impl InferType for Literal {
    fn infer_type(&self, _scope: &Scope, _errors: &mut ErrorCollector) -> TypeElement {
        match self {
            Literal::BooleanLiteral(_) => TypeElement::bool(),
            Literal::StringLiteral(_) => BasicTypeItem {
                ident: TypeIdent::new("String"),
                span: Span::default(),
            }
            .into(),
            Literal::NumberLiteral(n) => {
                // Infer the smallest integer type that can fit the value
                infer_number_type(&n.value)
            },
            Literal::NoneLiteral(_) => Box::new(OptionalTypeItem {
                inner: TypeElement::infer(),
                span: Span::default(),
            })
            .into(),
        }
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        match self {
            Literal::StringLiteral(_) => Ownership::UniqueOwned,
            Literal::NumberLiteral(_) | Literal::BooleanLiteral(_) => Ownership::UniqueOwned,
            Literal::NoneLiteral(_) => Ownership::UniqueOwned,
        }
    }
}

/// Infer the most appropriate integer type for a number literal
fn infer_number_type(value: &str) -> TypeElement {
    // Handle floating point numbers
    if value.contains('.') || value.contains('e') || value.contains('E') {
        return if value.ends_with("f32") {
            BasicTypeItem {
                ident: TypeIdent::new("Float"),
                span: Span::default(),
            }.into()
        } else {
            // For now, default to __Number for floats without explicit suffix
            // to maintain compatibility
            BasicTypeItem {
                ident: TypeIdent::new("__Number"),
                span: Span::default(),
            }.into()
        };
    }

    // Handle explicit type suffixes - these are unambiguous
    if let Some(type_name) = extract_type_suffix(value) {
        return BasicTypeItem {
            ident: TypeIdent::new(type_name),
            span: Span::default(),
        }.into();
    }

    // For integer literals without explicit suffix, use __Number to maintain
    // backward compatibility and let the type system resolve the correct type
    // based on context. This is more conservative than inferring specific types.
    BasicTypeItem {
        ident: TypeIdent::new("__Number"),
        span: Span::default(),
    }.into()
}

/// Extract type suffix from number literal (e.g., "42i32" -> Some("I32"))
fn extract_type_suffix(value: &str) -> Option<&'static str> {
    if value.ends_with("i8") { Some("I8") }
    else if value.ends_with("i16") { Some("I16") }
    else if value.ends_with("i32") { Some("I32") }
    else if value.ends_with("i64") { Some("I64") }
    else if value.ends_with("i128") { Some("I128") }
    else if value.ends_with("isize") { Some("ISize") }
    else if value.ends_with("u8") { Some("U8") }
    else if value.ends_with("u16") { Some("U16") }
    else if value.ends_with("u32") { Some("U32") }
    else if value.ends_with("u64") { Some("U64") }
    else if value.ends_with("u128") { Some("U128") }
    else if value.ends_with("usize") { Some("USize") }
    else if value.ends_with("f32") { Some("Float") }
    else if value.ends_with("f64") { Some("Double") }
    else { None }
}

impl InferType for InfixExpression {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        match self {
            InfixExpression::Logical(_) => TypeElement::bool(),
            InfixExpression::Arithmetic(e) => e.infer_type(scope, errors),
            InfixExpression::Collection(e) => e.infer_type(scope, errors),
            InfixExpression::Comparison(_) => TypeElement::bool(),
            InfixExpression::Member(e) => e.infer_type(scope, errors),
            InfixExpression::Unwrap(u) => u.infer_type(scope, errors),
            InfixExpression::Custom(_) => {
                errors.warning("Type inference for custom operators not yet implemented".to_string(), None);
                TypeElement::infer()
            }
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match self {
            InfixExpression::Logical(_) => Ownership::UniqueOwned,
            InfixExpression::Arithmetic(arith) => {
                // Check if the operands are copy types - if they are, arithmetic operations
                // can be copy, otherwise they should be owned
                let mut temp_errors = ErrorCollector::new();
                let lhs_type = arith.lhs.infer_type(scope, &mut temp_errors);
                let rhs_type = arith.rhs.infer_type(scope, &mut temp_errors);
                
                // Note: We don't emit warnings here as this is internal type checking
                // The actual error reporting will happen at a higher level
                
                let lhs_is_copy = ctx.mapping.is_copy(&lhs_type);
                let rhs_is_copy = ctx.mapping.is_copy(&rhs_type);
                
                // If both operands are copy types, the result can be copy (UniqueOwned)
                // If either operand is not copy, the result should be owned
                if lhs_is_copy && rhs_is_copy {
                    Ownership::UniqueOwned
                } else {
                    // Check the actual ownership of the operands
                    let lhs_owned = arith.lhs.infer_owned(ctx, scope);
                    let rhs_owned = arith.rhs.infer_owned(ctx, scope);
                    
                    match (lhs_owned, rhs_owned) {
                        (Ownership::UniqueOwned, Ownership::UniqueOwned) => Ownership::UniqueOwned,
                        (Ownership::SharedOwned, _) | (_, Ownership::SharedOwned) => Ownership::SharedOwned,
                        _ => Ownership::UniqueOwned,
                    }
                }
            }
            InfixExpression::Collection(collection) => collection.infer_owned(ctx, scope),
            InfixExpression::Comparison(_) => Ownership::UniqueOwned,
            InfixExpression::Member(mem) => {
                // Check if the field type is copy to distinguish copy and owned here
                let mut temp_errors = ErrorCollector::new();
                let field_type = mem.infer_type(scope, &mut temp_errors);
                
                // Note: We don't emit warnings here as this is internal type checking
                if ctx.mapping.is_copy(&field_type) {
                    Ownership::UniqueOwned
                } else {
                    // For non-copy fields, propagate the ownership of the receiver
                    mem.lhs.infer_owned(ctx, scope)
                }
            }
            InfixExpression::Unwrap(u) => u.infer_owned(ctx, scope),
            InfixExpression::Custom(_custom) => {
                eprintln!("TRANSPILER WARNING: Ownership inference for custom operators not yet implemented");
                Ownership::Borrowed
            }
        }
    }
}

impl InferType for InfixOperation<ArithmeticOperator> {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        let first = self.lhs.infer_type(scope, errors);
        let second = self.rhs.infer_type(scope, errors);

        if let (TypeElement::Plain(a), TypeElement::Plain(b)) = (&first, &second) {
            if a.ident != b.ident && !a.ident.is_intrinsic() && !b.ident.is_intrinsic() {
                // Check if types are compatible numeric types
                if !are_compatible_numeric_types(&a.ident, &b.ident) {
                    errors.error(TranspilerError::TypeMismatch {
                        expected: format!("compatible types for arithmetic operation"),
                        found: format!("'{:#?}' and '{:#?}'", a, b),
                    });
                    return TypeElement::infer();
                }
            }
        }

        let result = if first.is_infer() { second } else { first };

        result
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        eprintln!("TRANSPILER WARNING: Ownership inference for arithmetic operations not yet implemented");
        Ownership::UniqueOwned
    }
}

/// Check if two type identifiers represent compatible numeric types
fn are_compatible_numeric_types(a: &TypeIdent, b: &TypeIdent) -> bool {
    let integer_types = ["I8", "I16", "I32", "I64", "I128", "ISize", "Int", 
                        "U8", "U16", "U32", "U64", "U128", "USize", "UInt"];
    let float_types = ["Float", "Double"];
    
    let a_str = a.as_str();
    let b_str = b.as_str();
    
    // Both are integer types
    (integer_types.contains(&a_str) && integer_types.contains(&b_str)) ||
    // Both are float types  
    (float_types.contains(&a_str) && float_types.contains(&b_str)) ||
    // One is __Number (intrinsic) - these should be compatible with any numeric type
    a.is_intrinsic() || b.is_intrinsic()
}

impl InferType for InfixOperation<CollectionOperator> {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        let Self {
            lhs,
            operator,
            rhs: _,
        } = self;

        match operator {
            CollectionOperator::Concat => lhs.infer_type(scope, errors),
            CollectionOperator::Remove => lhs.infer_type(scope, errors),
            CollectionOperator::Contains => TypeElement::bool(),
        }
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        match self.operator {
            CollectionOperator::Concat
            | CollectionOperator::Remove
            | CollectionOperator::Contains => Ownership::UniqueOwned,
        }
    }
}

impl InferType for InfixOperation<MemberOperator> {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        let Self {
            lhs,
            operator: _,
            rhs,
        } = self;

        let receiver_type = lhs.infer_type(scope, errors);

        match receiver_type {
            TypeElement::Plain(ty) => {
                let Some(ty) = &scope.resolve_type(&ty.ident) else {
                    return TypeElement::infer();
                };
                let ref ty = ty.item;

                // println!("cargo::warning=resolved: {:?}", ty);
                match self.field_ident() {
                    Some(field) => {
                        // println!("cargo::warning=field: {:?}", field);

                        match ty {
                            TypeDecl::Tuple(_tuple) => {
                                errors.warning("Tuple member access not yet implemented".to_string(), None);
                                TypeElement::infer()
                            }
                            TypeDecl::Struct(st) => st
                                .members
                                .iter()
                                .find(|member| member.ident == *field)
                                .map(|member| member.r#type.clone())
                                .unwrap_or_else(|| {
                                    errors.error(TranspilerError::MemberAccessError {
                                        message: format!("struct does not have field: {field}"),
                                    });
                                    TypeElement::infer()
                                }),
                            TypeDecl::Enum(_) => {
                                errors.error(TranspilerError::EnumAccessError {
                                    message: "Enum cases are accessed with ::".to_string(),
                                });
                                TypeElement::infer()
                            }
                            TypeDecl::Alias(_) => {
                                // TODO: Handle Inference for alias types
                                TypeElement::infer()
                            }
                            TypeDecl::Empty(_) => {
                                errors.error(TranspilerError::MemberAccessError {
                                    message: "Cannot access member of empty type".to_string(),
                                });
                                TypeElement::infer()
                            }
                        }
                    }
                    None => {
                        if let ExpressionKind::FunctionCall(ref call) = rhs.kind {
                            println!("cargo::warning=functions: {:?}", scope.functions());
                            if let Some(func) =
                                scope.resolve_function(Some(ty.ident()), &call.identifier, &[])
                            {
                                func.item.signature.return_type.clone()
                            } else {
                                println!("cargo::warning=Function '{}' not found", call.identifier);
                                TypeElement::infer()
                            }
                        } else {
                            errors.error(TranspilerError::MemberAccessError {
                                message: "Member operator can only be used with function calls".to_string(),
                            });
                            TypeElement::infer()
                        }
                    }
                }
            }
            TypeElement::Optional(_) | TypeElement::Result(_) => {
                // TODO: Handle inference for optional and result types
                // TODO: Ultimately transition to a compiler error here
                //  that tells the user to use safe-call ?. or forward-error-call !.
                TypeElement::infer()
            }
            other => {
                if self.is_field() && !other.is_infer() {
                    errors.error(TranspilerError::MemberAccessError {
                        message: format!("Cannot access member of type {:?}", other),
                    });
                    TypeElement::infer()
                } else {
                    // TODO: take receiver type here and lookup function
                    TypeElement::infer()
                }
            }
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        let Self {
            lhs,
            operator: _,
            rhs: _,
        } = self;
        
        // Member access propagates the ownership of the receiver
        // If the receiver is borrowed, the member is also borrowed
        lhs.infer_owned(ctx, scope)
    }
}

impl InferType for InfixOperation<UnwrapOperator> {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        let ty = self.rhs.infer_type(scope, errors);

        match ty {
            TypeElement::Plain(_) if ty.is_number() => (),
            TypeElement::Infer(_) | TypeElement::Never(_) => (),
            _ => return ty,
        };

        match self.lhs.infer_type(scope, errors) {
            TypeElement::Optional(opt) => opt.inner.clone(),
            TypeElement::Result(res) => res.success.clone(),
            ty @ TypeElement::Infer(_) => ty.clone(),
            _ => {
                errors.error(TranspilerError::InvalidOperationOnType {
                    operation: "'?' operator".to_string(),
                    allowed_types: "result or optional types".to_string(),
                });
                TypeElement::infer()
            }
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match (
            self.rhs.infer_owned(ctx, scope),
            self.lhs.infer_owned(ctx, scope),
        ) {
            (lhs, rhs) if rhs == lhs => rhs,
            (
                Ownership::SharedOwned | Ownership::UniqueOwned,
                Ownership::SharedOwned | Ownership::UniqueOwned,
            ) => Ownership::SharedOwned,
            (Ownership::Borrowed, _) | (_, Ownership::Borrowed) => Ownership::Borrowed,
            _ => {
                // Fallback for incompatible ownership types in '?' operator
                Ownership::Borrowed
            }
        }
    }
}

impl InferType for PostfixExpression {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        match self {
            PostfixExpression::YeetExpression(yeet) => {
                let inner_type = yeet.inner.infer_type(scope, errors);
                
                // Check if return type is matching with the current function's return type
                match &inner_type {
                    TypeElement::Optional(opt) => {
                        // For optional types, check if the current function returns an optional
                        // that is compatible with the inner type
                        validate_yeet_return_type(scope, &inner_type);
                        opt.inner.clone()
                    }
                    TypeElement::Result(res) => {
                        // For result types, check if the current function returns a result
                        // with a compatible error type
                        validate_yeet_return_type(scope, &inner_type);
                        res.success.clone()
                    }
                    TypeElement::Infer(_) => TypeElement::infer(),
                    _ => {
                        errors.error(TranspilerError::InvalidOperationOnType {
                            operation: "Yeet operator".to_string(),
                            allowed_types: "result or optional types".to_string(),
                        });
                        TypeElement::infer()
                    }
                }
            }
            PostfixExpression::AccessExpression(access) => match access.base.infer_type(scope, errors) {
                TypeElement::Array(array) => array.elements,
                TypeElement::Dictionary(dict) => dict.value,
                TypeElement::OrderedDictionary(dict) => dict.value,
                TypeElement::Set(set) => set.elements,
                TypeElement::Infer(_) => TypeElement::infer(),
                _ => {
                    errors.error(TranspilerError::InvalidOperationOnType {
                        operation: "Access operator".to_string(),
                        allowed_types: "collection types".to_string(),
                    });
                    TypeElement::infer()
                }
            },
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match self {
            PostfixExpression::YeetExpression(yeet_expression) => {
                yeet_expression.inner.infer_owned(ctx, scope)
            }
            PostfixExpression::AccessExpression(access_expression) => {
                access_expression.base.infer_owned(ctx, scope)
            }
        }
    }
}

/// Validate that the yeet operator return type is compatible with the current function's return type
fn validate_yeet_return_type(scope: &Scope, yeet_type: &TypeElement) {
    let fn_return_type = &scope.fn_return;
    
    // If the function return type is not yet determined, we can't validate
    if fn_return_type.is_infer() || fn_return_type.is_void() {
        return;
    }
    
    match (yeet_type, fn_return_type) {
        // Optional -> Optional: check inner type compatibility
        (TypeElement::Optional(yeet_opt), TypeElement::Optional(fn_opt)) => {
            if !yeet_opt.inner.is_same(&fn_opt.inner) && !yeet_opt.inner.is_infer() && !fn_opt.inner.is_infer() {
                println!("cargo::warning=Yeet operator type mismatch: yielding {:?} but function returns {:?}", yeet_opt.inner, fn_opt.inner);
            }
        }
        // Result -> Result: check success and error type compatibility  
        (TypeElement::Result(yeet_res), TypeElement::Result(fn_res)) => {
            if !yeet_res.success.is_same(&fn_res.success) && !yeet_res.success.is_infer() && !fn_res.success.is_infer() {
                println!("cargo::warning=Yeet operator success type mismatch: yielding {:?} but function returns {:?}", yeet_res.success, fn_res.success);
            }
            
            // Check error type compatibility if both are specified
            if let (Some(yeet_err), Some(fn_err)) = (&yeet_res.error, &fn_res.error) {
                if !yeet_err.is_same(fn_err) && !yeet_err.is_infer() && !fn_err.is_infer() {
                    println!("cargo::warning=Yeet operator error type mismatch: yielding {:?} but function expects {:?}", yeet_err, fn_err);
                }
            }
        }
        // Optional -> Result or Result -> Optional: potentially incompatible
        (TypeElement::Optional(_), TypeElement::Result(_)) | 
        (TypeElement::Result(_), TypeElement::Optional(_)) => {
            println!("cargo::warning=Yeet operator type incompatibility: yielding {:?} but function returns {:?}", yeet_type, fn_return_type);
        }
        // Other combinations - we might want to allow automatic wrapping in the future
        _ => {
            // For now, just emit a warning for non-matching types
            if !yeet_type.is_infer() && !fn_return_type.is_infer() {
                println!("cargo::warning=Yeet operator return type validation: yielding {:?} from function returning {:?}", yeet_type, fn_return_type);
            }
        }
    }
}

impl InferType for CollectionLiteral {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        match self {
            CollectionLiteral::ArrayLiteral(array) => array.infer_type(scope, errors),
            CollectionLiteral::DictLiteral(dict) => dict.infer_type(scope, errors),
            CollectionLiteral::SetLiteral(set) => set.infer_type(scope, errors),
            CollectionLiteral::OrderedDictLiteral(ordered_dict) => ordered_dict.infer_type(scope, errors),
        }
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        eprintln!("TRANSPILER WARNING: Ownership inference for arithmetic operations not yet implemented");
        Ownership::UniqueOwned
    }
}

impl InferType for ArrayLiteral {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        let elements = infer_from_elements(&self.elements, scope, errors);
        Box::new(ArrayTypeItem {
            elements,
            span: Span::default(),
        })
        .into()
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        eprintln!("TRANSPILER WARNING: Ownership inference for arithmetic operations not yet implemented");
        Ownership::UniqueOwned
    }
}

impl InferType for SetLiteral {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        let elements = infer_from_elements(&self.elements, scope, errors);
        Box::new(SetTypeItem {
            elements,
            span: Span::default(),
        })
        .into()
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        eprintln!("TRANSPILER WARNING: Ownership inference for arithmetic operations not yet implemented");
        Ownership::UniqueOwned
    }
}

impl InferType for DictLiteral {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        let (key, value) = infer_dict_elements(&self.elements, scope, errors);
        Box::new(DictionaryTypeItem {
            key,
            value,
            span: Span::default(),
        })
        .into()
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        eprintln!("TRANSPILER WARNING: Ownership inference for arithmetic operations not yet implemented");
        Ownership::UniqueOwned
    }
}

impl InferType for OrderedDictLiteral {
    fn infer_type(&self, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement {
        let (key, value) = infer_dict_elements(&self.elements, scope, errors);
        Box::new(OrderedDictionaryTypeItem {
            key,
            value,
            span: Span::default(),
        })
        .into()
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        eprintln!("TRANSPILER WARNING: Ownership inference for arithmetic operations not yet implemented");
        Ownership::UniqueOwned
    }
}

fn infer_dict_elements(
    elements: &[DictLiteralElement],
    scope: &Scope,
    errors: &mut ErrorCollector,
) -> (TypeElement, TypeElement) {
    let keys = elements.iter().map(|element| &element.key).collect_vec();
    let values = elements.iter().map(|element| &element.value).collect_vec();

    let key = infer_from_elements(keys, scope, errors);
    let value = infer_from_elements(values, scope, errors);

    (key, value)
}

fn infer_from_elements<'a, I>(elements: I, scope: &Scope, errors: &mut ErrorCollector) -> TypeElement
where
    I: IntoIterator<Item = &'a Expression>,
{
    let inner = elements
        .into_iter()
        .map(|item| item.infer_type(scope, errors))
        .filter(|item| !item.is_infer())
        .unique()
        .collect::<Vec<_>>();

    match inner.len() {
        0 => TypeElement::infer(),
        1 => inner.into_iter().next().unwrap(),
        _ => {
            errors.error(TranspilerError::TypeMismatch {
                expected: "matching types in literal".to_string(),
                found: "multiple different types".to_string(),
            });
            TypeElement::infer()
        }
    }
}
