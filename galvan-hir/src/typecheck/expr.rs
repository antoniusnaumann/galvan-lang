//! Expression lowering: turns AST expressions into typed [`HirExpression`]s.

use galvan_ast::{
    BasicTypeItem, Closure, ClosureParameter, ClosureTypeItem, CollectionLiteral,
    ComparisonOperator, ConstructorCall, DeclModifier, DictLiteralElement, ElseExpression,
    EnumConstructor, Expression, ExpressionKind, FunctionCall, FunctionCallArg, Ident,
    InfixExpression, InfixOperation, Literal, MemberOperator, NeverTypeItem, OptionalTypeItem,
    Ownership, Param, PostfixExpression, ResultTypeItem, Span, TypeDecl, TypeElement, TypeIdent,
};
use galvan_resolver::Lookup;

use crate::builtins::{CheckBuiltins, BORROWED_ITERATOR_FNS};
use crate::error::TranspilerError;
use crate::hir::*;

use super::{concat_kind, types_compatible, Checker, Expected, Variable};

impl Checker<'_> {
    pub(crate) fn lower_expression(
        &mut self,
        expression: &Expression,
        expected: &Expected,
    ) -> HirExpression {
        let span = expression.span;
        let lowered = match &expression.kind {
            ExpressionKind::ElseExpression(else_expression) => {
                self.lower_else_expression(else_expression, expected, span)
            }
            ExpressionKind::FunctionCall(call) => self.lower_function_call(call, expected, span),
            ExpressionKind::Infix(infix) => self.lower_infix(infix, expected, span),
            ExpressionKind::Postfix(postfix) => self.lower_postfix(postfix, span),
            ExpressionKind::CollectionLiteral(literal) => self.lower_collection(literal, span),
            ExpressionKind::ConstructorCall(constructor) => {
                self.lower_constructor(constructor, span)
            }
            ExpressionKind::EnumConstructor(constructor) => {
                self.lower_enum_constructor(constructor, span)
            }
            ExpressionKind::EnumAccess(access) => HirExpression::new(
                HirExpressionKind::EnumAccess(HirEnumAccess {
                    target: access.target.clone(),
                    case: access.case.clone(),
                }),
                plain_type(access.target.clone()),
                Ownership::UniqueOwned,
                span,
            ),
            ExpressionKind::Literal(literal) => self.lower_literal(literal, span),
            ExpressionKind::Ident(ident) => self.lower_variable_expression(ident, span),
            ExpressionKind::Closure(closure) => self.lower_closure(closure, expected, false, span),
            ExpressionKind::Group(group) => {
                let inner = self.lower_expression(&group.inner, &Expected::free());
                let ty = inner.ty.clone();
                let ownership = inner.adjusted_ownership();
                HirExpression::new(
                    HirExpressionKind::Group(Box::new(inner)),
                    ty,
                    ownership,
                    span,
                )
            }
        };

        self.coerce(lowered, expected)
    }

    pub(crate) fn lower_modified_value(
        &mut self,
        expression: &Expression,
        modifier: Option<DeclModifier>,
        allow_ref: bool,
        context: &str,
    ) -> HirExpression {
        match modifier {
            Some(DeclModifier::Ref) if allow_ref => {
                let lowered = self.lower_expression(expression, &Expected::free());
                self.lower_ref_value(lowered, expression.span)
            }
            Some(modifier) => {
                self.errors.error_with_span(
                    TranspilerError::InvalidModifier {
                        modifier: modifier_name(modifier).to_string(),
                        context: context.to_string(),
                    },
                    Some(expression.span.into()),
                );
                self.lower_expression(expression, &Expected::free())
            }
            None => self.lower_expression(expression, &Expected::free()),
        }
    }

    pub(crate) fn lower_ref_value(&mut self, lowered: HirExpression, span: Span) -> HirExpression {
        if lowered.adjusted_ownership() != Ownership::Ref {
            self.errors.error_with_span(
                TranspilerError::IncompatibleOwnership {
                    message: "`ref` can only share a ref value".to_string(),
                },
                Some(span.into()),
            );
            return HirExpression::error("invalid ref modifier", span);
        }

        lowered.adjusted(Adjustment::ArcClone)
    }

    fn lower_variable_expression(&mut self, ident: &Ident, span: Span) -> HirExpression {
        match self.variable(ident, span) {
            Some(variable) => HirExpression::new(
                HirExpressionKind::Variable(ident.clone()),
                variable.ty,
                variable.ownership,
                span,
            ),
            None => HirExpression::new(
                HirExpressionKind::Variable(ident.clone()),
                TypeElement::infer(),
                Ownership::Borrowed,
                span,
            ),
        }
    }

    fn lower_literal(&mut self, literal: &Literal, span: Span) -> HirExpression {
        let (literal, ty) = match literal {
            Literal::BooleanLiteral(boolean) => {
                (HirLiteral::Boolean(boolean.value), TypeElement::bool())
            }
            Literal::NumberLiteral(number) => (
                HirLiteral::Number(number.value.clone()),
                infer_number_type(&number.value),
            ),
            Literal::CharLiteral(char_literal) => (
                HirLiteral::Char(char_literal.value),
                plain_type(TypeIdent::new("Char")),
            ),
            Literal::NoneLiteral(_) => (
                HirLiteral::None,
                TypeElement::Optional(Box::new(OptionalTypeItem {
                    inner: TypeElement::infer(),
                    span: Span::default(),
                })),
            ),
            Literal::StringLiteral(string) => {
                let interpolations = string
                    .interpolations
                    .iter()
                    .map(|interpolation| self.lower_interpolation(interpolation))
                    .collect();
                (
                    HirLiteral::String(HirStringLiteral {
                        value: string.value.clone(),
                        interpolations,
                    }),
                    plain_type(TypeIdent::new("String")),
                )
            }
        };

        HirExpression::new(
            HirExpressionKind::Literal(literal),
            ty,
            Ownership::UniqueOwned,
            span,
        )
    }

    /// Lowers a string interpolation argument. When parsing the interpolated
    /// source failed, the AST falls back to an identifier containing the raw
    /// expression source (e.g. `dog.name`); such identifiers are rendered
    /// verbatim instead of being resolved as variables.
    fn lower_interpolation(&mut self, interpolation: &Expression) -> HirExpression {
        if let ExpressionKind::Ident(ident) = &interpolation.kind {
            let is_fallback = ident
                .as_str()
                .contains(|c: char| !c.is_alphanumeric() && c != '_');
            if is_fallback {
                return HirExpression::new(
                    HirExpressionKind::Variable(ident.clone()),
                    TypeElement::infer(),
                    Ownership::Borrowed,
                    interpolation.span,
                );
            }
        }

        self.lower_expression(interpolation, &Expected::free())
    }

    // ------------------------------------------------------------------
    // Function calls (including the magic control-flow "functions")
    // ------------------------------------------------------------------

    fn lower_function_call(
        &mut self,
        call: &FunctionCall,
        expected: &Expected,
        span: Span,
    ) -> HirExpression {
        match call.identifier.as_str() {
            "panic" => self.lower_print(PrintKind::Panic, &call.arguments, span),
            "println" => self.lower_print(PrintKind::Println, &call.arguments, span),
            "print" => self.lower_print(PrintKind::Print, &call.arguments, span),
            "debug" => self.lower_print(PrintKind::Debug, &call.arguments, span),
            "if" => self.lower_if(call, expected, None, span),
            "for" => self.lower_for(call, expected, span),
            "try" => self.lower_try(call, expected, None, span),
            "assert" => self.lower_assert(call, span),
            name if BORROWED_ITERATOR_FNS.contains(&name) => {
                self.lower_borrowed_iterator_call(call, span)
            }
            _ => self.lower_call(None, &call.identifier, &call.arguments, span),
        }
    }

    fn lower_print(
        &mut self,
        kind: PrintKind,
        arguments: &[FunctionCallArg],
        span: Span,
    ) -> HirExpression {
        let args = arguments
            .iter()
            .map(|argument| {
                let lowered = self.lower_expression(&argument.expression, &Expected::free());
                self.coerce_unknown_argument(lowered)
            })
            .collect();

        let ty = match kind {
            PrintKind::Panic => TypeElement::Never(NeverTypeItem {
                span: Span::default(),
            }),
            _ => TypeElement::void(),
        };

        HirExpression::new(
            HirExpressionKind::Print(HirPrint { kind, args }),
            ty,
            Ownership::UniqueOwned,
            span,
        )
    }

    /// Lowers a call to a regular function or method, with or without a
    /// known signature
    fn lower_call(
        &mut self,
        receiver: Option<HirExpression>,
        ident: &Ident,
        arguments: &[FunctionCallArg],
        span: Span,
    ) -> HirExpression {
        if receiver.is_none() {
            if let Some(variable) = self.scopes.get(ident).cloned() {
                if let TypeElement::Closure(closure) = variable.ty {
                    let args = self.lower_closure_call_args(&closure.parameters, arguments);
                    return HirExpression::new(
                        HirExpressionKind::FunctionCall(HirFunctionCall {
                            ident: ident.clone(),
                            args,
                        }),
                        closure.return_ty.clone(),
                        Ownership::UniqueOwned,
                        span,
                    );
                }
            }
        }

        let receiver_ident = receiver.as_ref().and_then(|receiver| match &receiver.ty {
            TypeElement::Plain(basic) => Some(basic.ident.clone()),
            TypeElement::Parametric(parametric) => Some(parametric.base_type.clone()),
            TypeElement::Generic(generic) => Some(TypeIdent::new(generic.ident.as_str())),
            _ => None,
        });

        let lookup = self.lookup;
        let function = lookup
            .resolve_function(receiver_ident.as_ref(), ident, &[])
            // Extension functions on collection or generic receivers are
            // registered without a receiver type
            .or_else(|| lookup.resolve_function(None, ident, &[]));

        match function {
            Some(function) => {
                let signature = function.item.signature.clone();
                let args = self.lower_call_args(&signature.parameters.params, arguments);
                let ty = signature.return_type.clone();
                let kind = match receiver {
                    Some(receiver) => HirExpressionKind::MethodCall(Box::new(HirMethodCall {
                        receiver,
                        ident: ident.clone(),
                        args,
                    })),
                    None => HirExpressionKind::FunctionCall(HirFunctionCall {
                        ident: ident.clone(),
                        args,
                    }),
                };
                HirExpression::new(kind, ty, Ownership::UniqueOwned, span)
            }
            None => {
                let args = arguments
                    .iter()
                    .map(|argument| self.lower_unknown_argument(argument))
                    .collect();
                let kind = match receiver {
                    Some(receiver) => HirExpressionKind::MethodCall(Box::new(HirMethodCall {
                        receiver,
                        ident: ident.clone(),
                        args,
                    })),
                    None => HirExpressionKind::FunctionCall(HirFunctionCall {
                        ident: ident.clone(),
                        args,
                    }),
                };
                HirExpression::new(kind, TypeElement::infer(), Ownership::UniqueOwned, span)
            }
        }
    }

    /// Lowers arguments for a call with a known signature: every argument is
    /// coerced to the parameter type and the ownership implied by the
    /// parameter's declaration modifier
    fn lower_call_args(
        &mut self,
        params: &[Param],
        arguments: &[FunctionCallArg],
    ) -> Vec<HirExpression> {
        params
            .iter()
            .skip_while(|param| param.identifier.is_self())
            .zip(arguments)
            .map(|(param, argument)| {
                match argument.modifier {
                    // Explicit `mut` at the call site
                    Some(DeclModifier::Mut) => {
                        let expected =
                            Expected::with(param.param_type.clone(), Ownership::MutBorrowed);
                        self.lower_expression(&argument.expression, &expected)
                    }
                    // Explicit `ref` at the call site shares the reference
                    Some(DeclModifier::Ref) => {
                        let lowered =
                            self.lower_expression(&argument.expression, &Expected::free());
                        self.lower_ref_value(lowered, argument.expression.span)
                    }
                    Some(DeclModifier::Let) => {
                        self.errors.error(TranspilerError::InvalidModifier {
                            modifier: "let".to_string(),
                            context: "function arguments".to_string(),
                        });
                        HirExpression::error(
                            "invalid let modifier on argument",
                            argument.expression.span,
                        )
                    }
                    None => {
                        let ownership = match param.decl_modifier {
                            Some(DeclModifier::Let) => {
                                self.errors
                                    .warning("Let modifier not yet implemented".to_string(), None);
                                Ownership::Borrowed
                            }
                            Some(DeclModifier::Mut) => Ownership::MutBorrowed,
                            Some(DeclModifier::Ref) => {
                                self.errors
                                    .warning("Ref modifier not yet implemented".to_string(), None);
                                Ownership::Borrowed
                            }
                            None => {
                                if self.is_copy(&param.param_type) {
                                    Ownership::UniqueOwned
                                } else {
                                    Ownership::Borrowed
                                }
                            }
                        };
                        let expected = Expected::with(param.param_type.clone(), ownership);
                        self.lower_expression(&argument.expression, &expected)
                    }
                }
            })
            .collect()
    }

    fn lower_closure_call_args(
        &mut self,
        params: &[TypeElement],
        arguments: &[FunctionCallArg],
    ) -> Vec<HirExpression> {
        params
            .iter()
            .zip(arguments)
            .map(|(param_ty, argument)| match argument.modifier {
                Some(DeclModifier::Mut) => {
                    let expected = Expected::with(param_ty.clone(), Ownership::MutBorrowed);
                    self.lower_expression(&argument.expression, &expected)
                }
                Some(DeclModifier::Ref) => {
                    let lowered = self.lower_expression(&argument.expression, &Expected::free());
                    self.lower_ref_value(lowered, argument.expression.span)
                }
                Some(DeclModifier::Let) => {
                    self.errors.error(TranspilerError::InvalidModifier {
                        modifier: "let".to_string(),
                        context: "closure arguments".to_string(),
                    });
                    HirExpression::error(
                        "invalid let modifier on argument",
                        argument.expression.span,
                    )
                }
                None => {
                    let ownership = if self.is_copy(param_ty) {
                        Ownership::UniqueOwned
                    } else {
                        Ownership::Borrowed
                    };
                    let expected = Expected::with(param_ty.clone(), ownership);
                    self.lower_expression(&argument.expression, &expected)
                }
            })
            .collect()
    }

    /// Lowers an argument for a call whose signature is unknown (e.g. Rust
    /// standard library methods)
    fn lower_unknown_argument(&mut self, argument: &FunctionCallArg) -> HirExpression {
        match argument.modifier {
            Some(DeclModifier::Mut) => {
                let lowered = self.lower_expression(&argument.expression, &Expected::free());
                self.adjust_ownership(lowered, Ownership::MutBorrowed)
            }
            Some(DeclModifier::Ref) => {
                let lowered = self.lower_expression(&argument.expression, &Expected::free());
                self.lower_ref_value(lowered, argument.expression.span)
            }
            Some(DeclModifier::Let) => {
                self.errors.error(TranspilerError::InvalidModifier {
                    modifier: "let".to_string(),
                    context: "function arguments".to_string(),
                });
                HirExpression::error("invalid let modifier on argument", argument.expression.span)
            }
            None => {
                let lowered = self.lower_expression(&argument.expression, &Expected::free());
                self.coerce_unknown_argument(lowered)
            }
        }
    }

    /// Borrowing iterator adapters like `filter` take closures whose
    /// parameters are references; the closure parameters destructure them
    fn lower_borrowed_iterator_call(&mut self, call: &FunctionCall, span: Span) -> HirExpression {
        let args = call
            .arguments
            .iter()
            .map(|argument| match &argument.expression.kind {
                ExpressionKind::Closure(closure) => {
                    if argument.modifier.is_some() {
                        self.errors.error(TranspilerError::InvalidModifier {
                            modifier: "closure".to_string(),
                            context: "borrowed iterator functions".to_string(),
                        });
                        return HirExpression::error(
                            "invalid closure modifier",
                            argument.expression.span,
                        );
                    }
                    self.lower_closure(closure, &Expected::free(), true, argument.expression.span)
                }
                _ => self.lower_unknown_argument(argument),
            })
            .collect();

        HirExpression::new(
            HirExpressionKind::FunctionCall(HirFunctionCall {
                ident: call.identifier.clone(),
                args,
            }),
            TypeElement::infer(),
            Ownership::UniqueOwned,
            span,
        )
    }

    // ------------------------------------------------------------------
    // Control flow
    // ------------------------------------------------------------------

    fn lower_else_expression(
        &mut self,
        else_expression: &ElseExpression,
        expected: &Expected,
        span: Span,
    ) -> HirExpression {
        match &else_expression.receiver.kind {
            ExpressionKind::FunctionCall(call) if call.identifier.as_str() == "if" => {
                self.lower_if(call, expected, Some(else_expression), span)
            }
            ExpressionKind::FunctionCall(call) if call.identifier.as_str() == "try" => {
                self.lower_try(call, expected, Some(else_expression), span)
            }
            _ => self.lower_else_unwrap(else_expression, expected, span),
        }
    }

    /// `receiver else { block }`: unwrap an optional or fall back
    fn lower_else_unwrap(
        &mut self,
        else_expression: &ElseExpression,
        expected: &Expected,
        span: Span,
    ) -> HirExpression {
        let receiver = self.lower_expression(&else_expression.receiver, &Expected::free());

        let inner_ty = match &receiver.ty {
            TypeElement::Optional(optional) => optional.inner.clone(),
            other => other.clone(),
        };

        let value_expected = if expected.is_free() || expected.is_void() {
            Expected::owned(inner_ty.clone())
        } else {
            expected.clone()
        };

        let receiver_ownership = receiver.adjusted_ownership();
        let by_ref = receiver_ownership == Ownership::SharedOwned;
        let value_ownership = match receiver_ownership {
            Ownership::UniqueOwned => Ownership::UniqueOwned,
            _ => Ownership::Borrowed,
        };

        self.scopes.push();
        self.scopes.declare(Variable {
            ident: Ident::new("__value"),
            modifier: DeclModifier::Let,
            ty: inner_ty.clone(),
            ownership: value_ownership,
        });
        let value = HirExpression::new(
            HirExpressionKind::Variable(Ident::new("__value")),
            inner_ty.clone(),
            value_ownership,
            span,
        );
        let value = self.coerce(value, &value_expected);
        self.scopes.pop();

        let else_block = self.lower_block(&else_expression.block.body, &value_expected);

        let ty = if value_expected.is_free() {
            inner_ty
        } else {
            value_expected.ty.clone()
        };

        HirExpression::new(
            HirExpressionKind::ElseUnwrap(Box::new(HirElseUnwrap {
                receiver,
                by_ref,
                value,
                else_block,
            })),
            ty,
            Ownership::UniqueOwned,
            span,
        )
    }

    fn lower_if(
        &mut self,
        call: &FunctionCall,
        expected: &Expected,
        else_expression: Option<&ElseExpression>,
        span: Span,
    ) -> HirExpression {
        if call.arguments.len() != 2 {
            self.errors.error(TranspilerError::MissingArgument {
                operation: "if".to_string(),
                argument_type: "condition and body".to_string(),
            });
            return HirExpression::error("invalid if", span);
        }
        let Some(body) = closure_argument(&call.arguments[1]) else {
            self.errors.error(TranspilerError::MissingArgument {
                operation: "if".to_string(),
                argument_type: "body expression".to_string(),
            });
            return HirExpression::error("invalid if body", span);
        };

        let condition = self.lower_expression(
            &call.arguments[0].expression,
            &Expected::owned(TypeElement::bool()),
        );

        match else_expression {
            Some(else_expression) => {
                let then_block = self.lower_block(&body.block.body, expected);
                let else_block = self.lower_block(&else_expression.block.body, expected);

                let ty = unify_types(&then_block.ty, &else_block.ty).unwrap_or_else(|| {
                    self.errors.warning(
                        format!(
                            "Types of if and else expression don't match: if: {}, else: {}",
                            then_block.ty, else_block.ty
                        ),
                        Some(span.into()),
                    );
                    TypeElement::infer()
                });

                HirExpression::new(
                    HirExpressionKind::If(Box::new(HirIf {
                        condition,
                        then_block,
                        else_block: Some(else_block),
                        wraps_optional: false,
                    })),
                    ty,
                    Ownership::UniqueOwned,
                    span,
                )
            }
            None => {
                // In statement position the branch produces no value
                if expected.is_void() {
                    let then_block = self.lower_block(&body.block.body, &Expected::void());
                    return HirExpression::new(
                        HirExpressionKind::If(Box::new(HirIf {
                            condition,
                            then_block,
                            else_block: None,
                            wraps_optional: false,
                        })),
                        TypeElement::void(),
                        Ownership::UniqueOwned,
                        span,
                    );
                }

                // Without an else branch, an if expression evaluates to an
                // optional: the tail is wrapped in Some and codegen emits
                // `else { None }`
                let tail_expected = match &expected.ty {
                    TypeElement::Optional(optional) => Expected::owned(optional.inner.clone()),
                    _ => Expected::free(),
                };
                let mut then_block = self.lower_block(&body.block.body, &tail_expected);

                let wraps_optional =
                    !matches!(then_block.ty, TypeElement::Never(_) | TypeElement::Void(_));

                let ty = if wraps_optional {
                    if let Some(HirStatement::Expression(tail)) = then_block.statements.last_mut() {
                        let coerced = self
                            .ensure_owned(std::mem::replace(tail, HirExpression::error("", span)));
                        *tail = coerced.adjusted(Adjustment::WrapSome);
                    }
                    TypeElement::Optional(Box::new(OptionalTypeItem {
                        inner: then_block.ty.clone(),
                        span: Span::default(),
                    }))
                } else {
                    then_block.ty.clone()
                };

                HirExpression::new(
                    HirExpressionKind::If(Box::new(HirIf {
                        condition,
                        then_block,
                        else_block: None,
                        wraps_optional,
                    })),
                    ty,
                    Ownership::UniqueOwned,
                    span,
                )
            }
        }
    }

    fn lower_try(
        &mut self,
        call: &FunctionCall,
        expected: &Expected,
        else_expression: Option<&ElseExpression>,
        span: Span,
    ) -> HirExpression {
        if call.arguments.len() != 2 {
            self.errors.error(TranspilerError::MissingArgument {
                operation: "try".to_string(),
                argument_type: "condition and body".to_string(),
            });
            return HirExpression::error("invalid try", span);
        }
        let Some(body) = closure_argument(&call.arguments[1]) else {
            self.errors.error(TranspilerError::MissingArgument {
                operation: "try".to_string(),
                argument_type: "body expression".to_string(),
            });
            return HirExpression::error("invalid try body", span);
        };

        let condition = self.lower_expression(&call.arguments[0].expression, &Expected::free());

        let (kind, ok_ty, err_ty) = match &condition.ty {
            TypeElement::Optional(optional) => (TryKind::Optional, optional.inner.clone(), None),
            TypeElement::Result(result) => (
                TryKind::Result,
                result.success.clone(),
                Some(result.error.clone().unwrap_or_else(TypeElement::infer)),
            ),
            _ => (TryKind::Optional, TypeElement::infer(), None),
        };

        match else_expression {
            Some(else_expression) => self.lower_try_else(
                condition,
                kind,
                ok_ty,
                err_ty,
                body,
                else_expression,
                expected,
                span,
            ),
            None => {
                // Without an else branch, `try` lowers to the runtime support
                // function `r#try(condition, |binding| body)`
                let condition = self.coerce_unknown_argument(condition);

                self.scopes.push();
                let ok_bindings = body
                    .parameters
                    .iter()
                    .map(|parameter| {
                        self.scopes.declare(Variable {
                            ident: parameter.ident.clone(),
                            modifier: DeclModifier::Let,
                            ty: ok_ty.clone(),
                            ownership: Ownership::Borrowed,
                        });
                        parameter.ident.clone()
                    })
                    .collect();
                let block = self.lower_block(&body.block.body, &Expected::free());
                self.scopes.pop();

                HirExpression::new(
                    HirExpressionKind::Try(Box::new(HirTry {
                        condition,
                        kind,
                        ok_bindings,
                        err_binding: None,
                        body: block,
                        else_block: None,
                    })),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    span,
                )
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_try_else(
        &mut self,
        condition: HirExpression,
        kind: TryKind,
        ok_ty: TypeElement,
        err_ty: Option<TypeElement>,
        body: &Closure,
        else_expression: &ElseExpression,
        expected: &Expected,
        span: Span,
    ) -> HirExpression {
        // Clone the scrutinee when matching on it would move a value that is
        // used again later; borrow optionals/results that are already behind
        // a reference
        let condition_ownership = condition.adjusted_ownership();
        let condition = self.adjust_ownership(condition, condition_ownership);
        let condition = if condition_ownership == Ownership::Borrowed
            && matches!(
                condition.ty,
                TypeElement::Optional(_) | TypeElement::Result(_)
            ) {
            condition.adjusted(Adjustment::Borrow)
        } else {
            condition
        };

        let branch_expected = if expected.is_void() {
            Expected::void()
        } else {
            expected.clone()
        };

        let binding_ownership = |checker: &Self, ty: &TypeElement, scrutinee: Ownership| {
            // `Infer` is assumed to be copy here, matching the unwrap of
            // optionals with unknown inner type
            if checker.is_copy(ty) || ty.is_infer() {
                Ownership::UniqueOwned
            } else {
                match scrutinee {
                    Ownership::UniqueOwned | Ownership::SharedOwned => Ownership::SharedOwned,
                    _ => Ownership::Borrowed,
                }
            }
        };

        let scrutinee_ownership = condition.adjusted_ownership();

        self.scopes.push();
        let ok_ownership = binding_ownership(self, &ok_ty, scrutinee_ownership);
        let ok_bindings: Vec<Ident> = body
            .parameters
            .iter()
            .map(|parameter| {
                self.scopes.declare(Variable {
                    ident: parameter.ident.clone(),
                    modifier: DeclModifier::Let,
                    ty: ok_ty.clone(),
                    ownership: ok_ownership,
                });
                parameter.ident.clone()
            })
            .collect();
        let block = self.lower_block(&body.block.body, &branch_expected);
        self.scopes.pop();

        self.scopes.push();
        let err_binding = else_expression.parameters.first().map(|parameter| {
            let err_ty = err_ty.clone().unwrap_or_else(TypeElement::infer);
            let err_ownership = binding_ownership(self, &err_ty, scrutinee_ownership);
            self.scopes.declare(Variable {
                ident: parameter.ident.clone(),
                modifier: DeclModifier::Let,
                ty: err_ty,
                ownership: err_ownership,
            });
            parameter.ident.clone()
        });
        let else_block = self.lower_block(&else_expression.block.body, &branch_expected);
        self.scopes.pop();

        let ty = if branch_expected.is_free() {
            unify_types(&block.ty, &else_block.ty).unwrap_or_else(TypeElement::infer)
        } else {
            branch_expected.ty.clone()
        };

        HirExpression::new(
            HirExpressionKind::Try(Box::new(HirTry {
                condition,
                kind,
                ok_bindings,
                err_binding,
                body: block,
                else_block: Some(else_block),
            })),
            ty,
            Ownership::UniqueOwned,
            span,
        )
    }

    fn lower_for(&mut self, call: &FunctionCall, expected: &Expected, span: Span) -> HirExpression {
        if call.arguments.len() != 2 {
            self.errors.error(TranspilerError::MissingArgument {
                operation: "for".to_string(),
                argument_type: "iterable and body".to_string(),
            });
            return HirExpression::error("invalid for loop", span);
        }
        let Some(body) = closure_argument(&call.arguments[1]) else {
            self.errors.error(TranspilerError::MissingArgument {
                operation: "for".to_string(),
                argument_type: "body expression".to_string(),
            });
            return HirExpression::error("invalid for body", span);
        };

        let iterable = self.lower_expression(&call.arguments[0].expression, &Expected::free());

        let elem_ty = match &iterable.ty {
            TypeElement::Array(array) => array.elements.clone(),
            TypeElement::Set(set) => set.elements.clone(),
            TypeElement::Dictionary(_) | TypeElement::OrderedDictionary(_) => {
                self.errors.warning(
                    "For loop on dictionary not yet implemented".to_string(),
                    None,
                );
                TypeElement::infer()
            }
            TypeElement::Tuple(_) | TypeElement::Optional(_) | TypeElement::Result(_) => {
                TypeElement::infer()
            }
            TypeElement::Infer(_) | TypeElement::Void(_) => TypeElement::infer(),
            _ => {
                self.errors
                    .warning("For loop on type that is not an iterator".to_string(), None);
                TypeElement::infer()
            }
        };

        // Borrow iterated locals so the loop does not consume them
        let iterable = match (&iterable.kind, iterable.adjusted_ownership()) {
            (HirExpressionKind::Variable(_), Ownership::SharedOwned) => {
                iterable.adjusted(Adjustment::Borrow)
            }
            (HirExpressionKind::Variable(_), Ownership::Ref) => {
                iterable.adjusted(Adjustment::LockRef)
            }
            _ => iterable,
        };

        let is_range = matches!(iterable.kind, HirExpressionKind::Range(_));
        let bind_by_ref = self.is_copy(&elem_ty) && !is_range && !elem_ty.is_infer();

        let collect = if expected.is_void() {
            None
        } else {
            Some(iteration_type(&self.fn_return, &expected.ty))
        };

        let element_ownership = if self.is_copy(&elem_ty) {
            Ownership::UniqueOwned
        } else {
            Ownership::Borrowed
        };

        self.scopes.push();
        let bindings: Vec<Ident> = if body.parameters.is_empty() {
            // Implicit `it` parameter
            self.scopes.declare(Variable {
                ident: Ident::new("it"),
                modifier: DeclModifier::Let,
                ty: elem_ty.clone(),
                ownership: element_ownership,
            });
            vec![]
        } else {
            body.parameters
                .iter()
                .map(|parameter| {
                    self.scopes.declare(Variable {
                        ident: parameter.ident.clone(),
                        modifier: DeclModifier::Let,
                        ty: elem_ty.clone(),
                        ownership: element_ownership,
                    });
                    parameter.ident.clone()
                })
                .collect()
        };

        let body_expected = match &collect {
            Some(collect_ty) => Expected::owned(collect_ty.clone()),
            None => Expected::void(),
        };
        let block = self.lower_block(&body.block.body, &body_expected);
        self.scopes.pop();

        let ty = match &collect {
            Some(collect_ty) => TypeElement::Array(Box::new(galvan_ast::ArrayTypeItem {
                elements: collect_ty.clone(),
                span: Span::default(),
            })),
            None => TypeElement::void(),
        };

        HirExpression::new(
            HirExpressionKind::For(Box::new(HirFor {
                bindings,
                bind_by_ref,
                iterable,
                body: block,
                collect,
            })),
            ty,
            Ownership::UniqueOwned,
            span,
        )
    }

    fn lower_assert(&mut self, call: &FunctionCall, span: Span) -> HirExpression {
        let assert = match call.arguments.first() {
            Some(FunctionCallArg {
                modifier,
                expression:
                    Expression {
                        kind: ExpressionKind::Infix(infix),
                        ..
                    },
            }) if infix.is_comparison() => {
                if modifier.is_some() {
                    self.errors.error(TranspilerError::InvalidModifier {
                        modifier: "assert".to_string(),
                        context: "comparison expressions".to_string(),
                    });
                    return HirExpression::error("invalid assert modifier", span);
                }
                let InfixExpression::Comparison(comparison) = infix.as_ref() else {
                    unreachable!()
                };

                let rest = call
                    .arguments
                    .iter()
                    .skip(1)
                    .map(|argument| self.lower_unknown_argument(argument))
                    .collect::<Vec<_>>();

                match comparison.operator {
                    ComparisonOperator::Equal => {
                        let (lhs, rhs) = self.unify_assert_sides(&comparison.lhs, &comparison.rhs);
                        HirAssert::Eq(lhs, rhs, rest)
                    }
                    ComparisonOperator::NotEqual => {
                        let (lhs, rhs) = self.unify_assert_sides(&comparison.lhs, &comparison.rhs);
                        HirAssert::Ne(lhs, rhs, rest)
                    }
                    _ => HirAssert::Truthy(
                        call.arguments
                            .iter()
                            .map(|argument| {
                                self.lower_expression(&argument.expression, &Expected::free())
                            })
                            .collect(),
                    ),
                }
            }
            Some(_) => HirAssert::Truthy(
                call.arguments
                    .iter()
                    .map(|argument| self.lower_expression(&argument.expression, &Expected::free()))
                    .collect(),
            ),
            None => {
                self.errors.error(TranspilerError::InvalidSyntax {
                    message: "Assert requires a condition or comparison expression".to_string(),
                });
                return HirExpression::error("invalid assert arguments", span);
            }
        };

        HirExpression::new(
            HirExpressionKind::Assert(Box::new(assert)),
            TypeElement::void(),
            Ownership::UniqueOwned,
            span,
        )
    }

    /// Brings both sides of an assert comparison to the same type and
    /// reference level
    fn unify_assert_sides(
        &mut self,
        lhs: &Expression,
        rhs: &Expression,
    ) -> (HirExpression, HirExpression) {
        let mut lhs = self.lower_expression(lhs, &Expected::free());
        let mut rhs = self.lower_expression(rhs, &Expected::free());

        // `ref` variables compare by their locked value
        if lhs.adjusted_ownership() == Ownership::Ref {
            lhs = lhs
                .adjusted(Adjustment::LockRef)
                .adjusted(Adjustment::Deref);
        }
        if rhs.adjusted_ownership() == Ownership::Ref {
            rhs = rhs
                .adjusted(Adjustment::LockRef)
                .adjusted(Adjustment::Deref);
        }

        let has_number =
            lhs.ty.is_number() || rhs.ty.is_number() || lhs.ty.is_infer() || rhs.ty.is_infer();

        // Wrap a plain value when the other side is an optional or result
        match (&lhs.ty, &rhs.ty) {
            (a, b) if types_compatible(a, b) => {}
            (TypeElement::Optional(optional), other)
                if types_compatible(&optional.inner, other) =>
            {
                rhs = rhs.adjusted(Adjustment::WrapSome);
            }
            (other, TypeElement::Optional(optional))
                if types_compatible(&optional.inner, other) =>
            {
                lhs = lhs.adjusted(Adjustment::WrapSome);
            }
            (TypeElement::Result(result), other) if types_compatible(&result.success, other) => {
                rhs = rhs.adjusted(Adjustment::WrapOk);
            }
            (other, TypeElement::Result(result)) if types_compatible(&result.success, other) => {
                lhs = lhs.adjusted(Adjustment::WrapOk);
            }
            _ => {}
        }

        use Ownership::*;
        if has_number {
            // References to number literals break type unification, so
            // dereference single borrowed variables instead of borrowing
            // the other side
            let lhs_deref = matches!(lhs.ownership, Borrowed | MutBorrowed)
                && matches!(lhs.kind, HirExpressionKind::Variable(_));
            let rhs_deref = matches!(rhs.ownership, Borrowed | MutBorrowed)
                && matches!(rhs.kind, HirExpressionKind::Variable(_));
            match (lhs_deref, rhs_deref) {
                (true, false) => lhs = lhs.adjusted(Adjustment::Deref),
                (false, true) => rhs = rhs.adjusted(Adjustment::Deref),
                _ => {}
            }
        } else {
            match (lhs.adjusted_ownership(), rhs.adjusted_ownership()) {
                (SharedOwned | UniqueOwned, Borrowed | MutBorrowed) => {
                    lhs = lhs.adjusted(Adjustment::Borrow);
                }
                (Borrowed | MutBorrowed, SharedOwned | UniqueOwned) => {
                    rhs = rhs.adjusted(Adjustment::Borrow);
                }
                _ => {}
            }
        }

        (lhs, rhs)
    }

    // ------------------------------------------------------------------
    // Infix operations
    // ------------------------------------------------------------------

    fn lower_infix(
        &mut self,
        infix: &InfixExpression,
        expected: &Expected,
        span: Span,
    ) -> HirExpression {
        match infix {
            InfixExpression::Logical(operation) => {
                let lhs = self.lower_expression(&operation.lhs, &Expected::free());
                let rhs = self.lower_expression(&operation.rhs, &Expected::free());
                HirExpression::new(
                    HirExpressionKind::Logical(Box::new(HirBinary {
                        lhs,
                        operator: operation.operator.clone(),
                        rhs,
                    })),
                    TypeElement::bool(),
                    Ownership::UniqueOwned,
                    span,
                )
            }
            InfixExpression::Arithmetic(operation) => {
                let lhs = self.lower_expression(&operation.lhs, &Expected::free());
                let rhs = self.lower_expression(&operation.rhs, &Expected::free());

                if let (TypeElement::Plain(a), TypeElement::Plain(b)) = (&lhs.ty, &rhs.ty) {
                    if !are_compatible_numeric_types(&a.ident, &b.ident) {
                        self.errors.warning(
                            format!(
                                "Type mismatch in arithmetic operation: {} {:?} {}",
                                lhs.ty, operation.operator, rhs.ty
                            ),
                            Some(operation.rhs.span.into()),
                        );
                    }
                }

                let ty = if lhs.ty.is_infer() || lhs.ty.is_number() {
                    rhs.ty.clone()
                } else {
                    lhs.ty.clone()
                };

                HirExpression::new(
                    HirExpressionKind::Arithmetic(Box::new(HirBinary {
                        lhs,
                        operator: operation.operator.clone(),
                        rhs,
                    })),
                    ty,
                    Ownership::UniqueOwned,
                    span,
                )
            }
            InfixExpression::Comparison(operation) => {
                let lhs = self.lower_expression(&operation.lhs, &Expected::free());
                let rhs = self.lower_expression(&operation.rhs, &Expected::free());
                HirExpression::new(
                    HirExpressionKind::Comparison(Box::new(HirBinary {
                        lhs,
                        operator: operation.operator.clone(),
                        rhs,
                    })),
                    TypeElement::bool(),
                    Ownership::UniqueOwned,
                    span,
                )
            }
            InfixExpression::Range(operation) => {
                let lhs = self.lower_expression(&operation.lhs, &Expected::free());
                let rhs = self.lower_expression(&operation.rhs, &Expected::free());
                let ty = TypeElement::Array(Box::new(galvan_ast::ArrayTypeItem {
                    elements: lhs.ty.clone(),
                    span: Span::default(),
                }));
                HirExpression::new(
                    HirExpressionKind::Range(Box::new(HirBinary {
                        lhs,
                        operator: operation.operator.clone(),
                        rhs,
                    })),
                    ty,
                    Ownership::UniqueOwned,
                    span,
                )
            }
            InfixExpression::Collection(operation) => {
                let lhs = self.lower_expression(&operation.lhs, &Expected::free());
                let rhs = self.lower_expression(&operation.rhs, &Expected::free());
                let (operator, rhs) = match operation.operator {
                    galvan_ast::CollectionOperator::Concat => {
                        let kind = concat_kind(&lhs.ty, &rhs.ty);
                        let rhs = self.coerce_concat_value(&lhs.ty, kind, rhs, false);
                        (CollectionOperator::Concat(kind), rhs)
                    }
                    galvan_ast::CollectionOperator::Remove => (CollectionOperator::Remove, rhs),
                    galvan_ast::CollectionOperator::Contains => (CollectionOperator::Contains, rhs),
                };
                let ty = match operator {
                    CollectionOperator::Concat(_) | CollectionOperator::Remove => lhs.ty.clone(),
                    CollectionOperator::Contains => TypeElement::bool(),
                };
                HirExpression::new(
                    HirExpressionKind::CollectionOp(Box::new(HirBinary { lhs, operator, rhs })),
                    ty,
                    Ownership::UniqueOwned,
                    span,
                )
            }
            InfixExpression::Member(operation) => self.lower_member(operation, expected, span),
            InfixExpression::Custom(_) => {
                self.errors.warning(
                    "Custom infix operators are not yet implemented".to_string(),
                    Some(span.into()),
                );
                HirExpression::error("custom infix operators are not implemented", span)
            }
        }
    }

    fn lower_member(
        &mut self,
        operation: &InfixOperation<MemberOperator>,
        _expected: &Expected,
        span: Span,
    ) -> HirExpression {
        match operation.operator {
            MemberOperator::Dot => match &operation.rhs.kind {
                ExpressionKind::FunctionCall(call) => {
                    let receiver = self.lower_expression(&operation.lhs, &Expected::free());
                    self.lower_call(Some(receiver), &call.identifier, &call.arguments, span)
                }
                ExpressionKind::Ident(field) => {
                    let receiver = self.lower_expression(&operation.lhs, &Expected::free());
                    let field_ty = self.field_type(&receiver.ty, field, span);
                    let ownership = if self.is_copy(&field_ty) {
                        Ownership::UniqueOwned
                    } else {
                        receiver.adjusted_ownership()
                    };
                    HirExpression::new(
                        HirExpressionKind::FieldAccess(Box::new(HirFieldAccess {
                            receiver,
                            field: field.clone(),
                        })),
                        field_ty,
                        ownership,
                        span,
                    )
                }
                _ => {
                    self.errors.error(TranspilerError::MemberAccessError {
                        message: "Member operator can only be used with fields or function calls"
                            .to_string(),
                    });
                    HirExpression::error("invalid member access", span)
                }
            },
            MemberOperator::SafeCall => self.lower_safe_access(operation, span),
        }
    }

    /// Resolves the type of a field on a receiver type
    fn field_type(&mut self, receiver_ty: &TypeElement, field: &Ident, span: Span) -> TypeElement {
        let type_ident = match receiver_ty {
            TypeElement::Plain(basic) => basic.ident.clone(),
            TypeElement::Parametric(parametric) => parametric.base_type.clone(),
            TypeElement::Optional(_) | TypeElement::Result(_) => {
                self.errors.error(TranspilerError::MemberAccessError {
                    message:
                        "Should use safe-call operator '?.' or error forwarding '!' on optional and result types"
                            .to_string(),
                });
                return TypeElement::infer();
            }
            _ => return TypeElement::infer(),
        };

        let lookup = self.lookup;
        let Some(decl) = lookup.resolve_type(&type_ident) else {
            return TypeElement::infer();
        };

        match &decl.item {
            TypeDecl::Struct(decl) => decl
                .members
                .iter()
                .find(|member| member.ident == *field)
                .map(|member| member.r#type.clone())
                .unwrap_or_else(|| {
                    self.errors.error(TranspilerError::MemberAccessError {
                        message: format!("struct does not have field: {field}"),
                    });
                    TypeElement::infer()
                }),
            TypeDecl::Tuple(_) => {
                self.errors.warning(
                    "Tuple member access not yet implemented".to_string(),
                    Some(span.into()),
                );
                TypeElement::infer()
            }
            TypeDecl::Enum(_) => {
                self.errors.error(TranspilerError::EnumAccessError {
                    message: "Enum cases are accessed with ::".to_string(),
                });
                TypeElement::infer()
            }
            // TODO: Handle inference for alias types
            TypeDecl::Alias(_) => TypeElement::infer(),
            TypeDecl::Empty(_) => {
                self.errors.error(TranspilerError::MemberAccessError {
                    message: "Cannot access member of empty type".to_string(),
                });
                TypeElement::infer()
            }
        }
    }

    fn lower_safe_access(
        &mut self,
        operation: &InfixOperation<MemberOperator>,
        span: Span,
    ) -> HirExpression {
        let receiver = self.lower_expression(&operation.lhs, &Expected::free());

        let (inner_ty, err_ty) = match &receiver.ty {
            TypeElement::Optional(optional) => (optional.inner.clone(), None),
            TypeElement::Result(result) => (result.success.clone(), Some(result.error.clone())),
            _ => (TypeElement::infer(), None),
        };

        let (access, access_ty) = match &operation.rhs.kind {
            ExpressionKind::Ident(field) => {
                let field_ty = self.field_type(&inner_ty, field, span);
                (SafeAccessKind::Field(field.clone()), field_ty)
            }
            ExpressionKind::FunctionCall(call) => {
                let receiver_ident = match &inner_ty {
                    TypeElement::Plain(basic) => Some(basic.ident.clone()),
                    TypeElement::Parametric(parametric) => Some(parametric.base_type.clone()),
                    _ => None,
                };
                let lookup = self.lookup;
                let function = lookup
                    .resolve_function(receiver_ident.as_ref(), &call.identifier, &[])
                    .or_else(|| lookup.resolve_function(None, &call.identifier, &[]));
                match function {
                    Some(function) => {
                        let signature = function.item.signature.clone();
                        let args =
                            self.lower_call_args(&signature.parameters.params, &call.arguments);
                        (
                            SafeAccessKind::Call(call.identifier.clone(), args),
                            signature.return_type.clone(),
                        )
                    }
                    None => {
                        let args = call
                            .arguments
                            .iter()
                            .map(|argument| self.lower_unknown_argument(argument))
                            .collect();
                        (
                            SafeAccessKind::Call(call.identifier.clone(), args),
                            TypeElement::infer(),
                        )
                    }
                }
            }
            _ => {
                self.errors.error(TranspilerError::MemberAccessError {
                    message: "Safe-call operator can only be used with fields or function calls"
                        .to_string(),
                });
                return HirExpression::error("invalid safe call", span);
            }
        };

        let style = match receiver.adjusted_ownership() {
            Ownership::SharedOwned | Ownership::MutBorrowed => SafeAccessStyle::RefClone,
            Ownership::UniqueOwned => SafeAccessStyle::Move,
            Ownership::Borrowed => SafeAccessStyle::Clone,
            Ownership::Ref => {
                self.errors.warning(
                    "Safe-call on ref variables is not implemented yet".to_string(),
                    Some(span.into()),
                );
                SafeAccessStyle::RefClone
            }
        };

        let ty = match err_ty {
            Some(error) => TypeElement::Result(Box::new(ResultTypeItem {
                success: access_ty,
                error,
                span: Span::default(),
            })),
            None => TypeElement::Optional(Box::new(OptionalTypeItem {
                inner: access_ty,
                span: Span::default(),
            })),
        };

        HirExpression::new(
            HirExpressionKind::SafeAccess(Box::new(HirSafeAccess {
                receiver,
                access,
                style,
            })),
            ty,
            Ownership::UniqueOwned,
            span,
        )
    }

    // ------------------------------------------------------------------
    // Postfix operations
    // ------------------------------------------------------------------

    fn lower_postfix(&mut self, postfix: &PostfixExpression, span: Span) -> HirExpression {
        match postfix {
            PostfixExpression::YeetExpression(yeet) => {
                let inner = self.lower_expression(&yeet.inner, &Expected::free());
                let ty = match &inner.ty {
                    TypeElement::Optional(optional) => {
                        self.validate_yeet_return_type(&inner.ty);
                        optional.inner.clone()
                    }
                    TypeElement::Result(result) => {
                        self.validate_yeet_return_type(&inner.ty);
                        result.success.clone()
                    }
                    TypeElement::Infer(_) => TypeElement::infer(),
                    _ => {
                        self.errors.error(TranspilerError::InvalidOperationOnType {
                            operation: "Yeet operator".to_string(),
                            allowed_types: "result or optional types".to_string(),
                        });
                        TypeElement::infer()
                    }
                };
                let ownership = inner.adjusted_ownership();
                HirExpression::new(
                    HirExpressionKind::Yeet(Box::new(inner)),
                    ty,
                    ownership,
                    span,
                )
            }
            PostfixExpression::AccessExpression(access) => {
                let base = self.lower_expression(&access.base, &Expected::free());
                let index = self.lower_expression(&access.index, &Expected::free());
                let ty = match &base.ty {
                    TypeElement::Array(array) => array.elements.clone(),
                    TypeElement::Dictionary(dict) => dict.value.clone(),
                    TypeElement::OrderedDictionary(dict) => dict.value.clone(),
                    TypeElement::Set(set) => set.elements.clone(),
                    TypeElement::Infer(_) => TypeElement::infer(),
                    _ => {
                        self.errors.error(TranspilerError::InvalidOperationOnType {
                            operation: "index access".to_string(),
                            allowed_types: "collection types".to_string(),
                        });
                        TypeElement::infer()
                    }
                };
                let ownership = if self.is_copy(&ty) {
                    Ownership::UniqueOwned
                } else {
                    base.adjusted_ownership()
                };
                HirExpression::new(
                    HirExpressionKind::Index(Box::new(HirIndex { base, index })),
                    ty,
                    ownership,
                    span,
                )
            }
        }
    }

    /// Validates that a yeeted (`!`) error type is compatible with the
    /// current function's return type
    fn validate_yeet_return_type(&mut self, yeet_ty: &TypeElement) {
        let fn_return = &self.fn_return;
        if fn_return.is_infer() || fn_return.is_void() {
            return;
        }

        match (yeet_ty, fn_return) {
            (TypeElement::Optional(yeet), TypeElement::Optional(ret)) => {
                if !types_compatible(&yeet.inner, &ret.inner) {
                    self.errors.warning(
                        format!(
                            "Yeet operator type mismatch: yielding {} but function returns {}",
                            yeet.inner, ret.inner
                        ),
                        None,
                    );
                }
            }
            (TypeElement::Result(yeet), TypeElement::Result(ret)) => {
                if !types_compatible(&yeet.success, &ret.success) {
                    self.errors.warning(
                        format!(
                            "Yeet operator success type mismatch: yielding {} but function returns {}",
                            yeet.success, ret.success
                        ),
                        None,
                    );
                }
                if let (Some(yeet_err), Some(ret_err)) = (&yeet.error, &ret.error) {
                    if !types_compatible(yeet_err, ret_err) {
                        self.errors.warning(
                            format!(
                                "Yeet operator error type mismatch: yielding {} but function expects {}",
                                yeet_err, ret_err
                            ),
                            None,
                        );
                    }
                }
            }
            (TypeElement::Optional(_), TypeElement::Result(_))
            | (TypeElement::Result(_), TypeElement::Optional(_)) => {
                self.errors.warning(
                    format!(
                        "Yeet operator type incompatibility: yielding {} but function returns {}",
                        yeet_ty, fn_return
                    ),
                    None,
                );
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------
    // Literals, constructors and closures
    // ------------------------------------------------------------------

    fn lower_collection(&mut self, literal: &CollectionLiteral, span: Span) -> HirExpression {
        let (collection, ty) = match literal {
            CollectionLiteral::ArrayLiteral(array) => {
                let (elements, elem_ty) = self.lower_collection_elements(&array.elements);
                (
                    HirCollection::Array(elements),
                    TypeElement::Array(Box::new(galvan_ast::ArrayTypeItem {
                        elements: elem_ty,
                        span: Span::default(),
                    })),
                )
            }
            CollectionLiteral::SetLiteral(set) => {
                let (elements, elem_ty) = self.lower_collection_elements(&set.elements);
                (
                    HirCollection::Set(elements),
                    TypeElement::Set(Box::new(galvan_ast::SetTypeItem {
                        elements: elem_ty,
                        span: Span::default(),
                    })),
                )
            }
            CollectionLiteral::DictLiteral(dict) => {
                let (elements, key_ty, value_ty) = self.lower_dict_elements(&dict.elements);
                (
                    HirCollection::Dict(elements),
                    TypeElement::Dictionary(Box::new(galvan_ast::DictionaryTypeItem {
                        key: key_ty,
                        value: value_ty,
                        span: Span::default(),
                    })),
                )
            }
            CollectionLiteral::OrderedDictLiteral(dict) => {
                let (elements, key_ty, value_ty) = self.lower_dict_elements(&dict.elements);
                (
                    HirCollection::OrderedDict(elements),
                    TypeElement::OrderedDictionary(Box::new(
                        galvan_ast::OrderedDictionaryTypeItem {
                            key: key_ty,
                            value: value_ty,
                            span: Span::default(),
                        },
                    )),
                )
            }
        };

        HirExpression::new(
            HirExpressionKind::Collection(collection),
            ty,
            Ownership::UniqueOwned,
            span,
        )
    }

    fn lower_collection_elements(
        &mut self,
        elements: &[Expression],
    ) -> (Vec<HirExpression>, TypeElement) {
        let lowered: Vec<HirExpression> = elements
            .iter()
            .map(|element| self.lower_expression(element, &Expected::free()))
            .collect();
        let ty = self.unify_element_types(lowered.iter().map(|element| &element.ty));
        (lowered, ty)
    }

    fn lower_dict_elements(
        &mut self,
        elements: &[DictLiteralElement],
    ) -> (Vec<HirDictElement>, TypeElement, TypeElement) {
        let lowered: Vec<HirDictElement> = elements
            .iter()
            .map(|element| HirDictElement {
                key: self.lower_expression(&element.key, &Expected::free()),
                value: self.lower_expression(&element.value, &Expected::free()),
            })
            .collect();
        let key_ty = self.unify_element_types(lowered.iter().map(|element| &element.key.ty));
        let value_ty = self.unify_element_types(lowered.iter().map(|element| &element.value.ty));
        (lowered, key_ty, value_ty)
    }

    fn unify_element_types<'t>(
        &mut self,
        types: impl Iterator<Item = &'t TypeElement>,
    ) -> TypeElement {
        let mut unified: Option<TypeElement> = None;
        for ty in types {
            if ty.is_infer() || ty.is_number() {
                continue;
            }
            match &unified {
                None => unified = Some(ty.clone()),
                Some(current) if types_compatible(current, ty) => {}
                Some(_) => {
                    self.errors.error(TranspilerError::TypeMismatch {
                        expected: "matching types in literal".to_string(),
                        found: "multiple different types".to_string(),
                    });
                    return TypeElement::infer();
                }
            }
        }
        unified.unwrap_or_else(TypeElement::infer)
    }

    fn lower_constructor(&mut self, constructor: &ConstructorCall, span: Span) -> HirExpression {
        let lookup = self.lookup;
        let type_decl = lookup.resolve_type(&constructor.identifier);

        let args = match type_decl.map(|decl| &decl.item) {
            Some(TypeDecl::Struct(decl)) => {
                let mut args = Vec::with_capacity(decl.members.len());
                for member in &decl.members {
                    let is_ref_field = matches!(member.decl_modifier, Some(DeclModifier::Ref));
                    let provided = constructor
                        .arguments
                        .iter()
                        .find(|argument| argument.ident == member.ident);
                    let value = match provided {
                        Some(argument) => {
                            let mut value = self.lower_modified_value(
                                &argument.expression,
                                argument.modifier,
                                is_ref_field,
                                "constructor arguments",
                            );
                            if !is_ref_field || argument.modifier != Some(DeclModifier::Ref) {
                                let expected = Expected::owned(member.r#type.clone());
                                value = self.coerce(value, &expected);
                            }
                            value
                        }
                        None => match &member.default_value {
                            Some(default) => {
                                let value = self.lower_expression(default, &Expected::free());
                                let expected = Expected::owned(member.r#type.clone());
                                self.coerce(value, &expected)
                            }
                            None => {
                                self.errors.error(TranspilerError::ArgumentCountMismatch {
                                    name: format!("{}()", constructor.identifier.as_str()),
                                    expected: decl.members.len(),
                                    found: constructor.arguments.len(),
                                });
                                HirExpression::error("missing field", span)
                            }
                        },
                    };
                    args.push(HirConstructorArg {
                        field: member.ident.clone(),
                        value,
                        store_as_ref: is_ref_field,
                    });
                }
                args
            }
            _ => constructor
                .arguments
                .iter()
                .map(|argument| {
                    let value = self.lower_modified_value(
                        &argument.expression,
                        argument.modifier,
                        false,
                        "constructor arguments",
                    );
                    let value = self.ensure_owned(value);
                    HirConstructorArg {
                        field: argument.ident.clone(),
                        value,
                        store_as_ref: false,
                    }
                })
                .collect(),
        };

        HirExpression::new(
            HirExpressionKind::ConstructorCall(HirConstructorCall {
                ident: constructor.identifier.clone(),
                args,
            }),
            plain_type(constructor.identifier.clone()),
            Ownership::UniqueOwned,
            span,
        )
    }

    fn lower_enum_constructor(
        &mut self,
        constructor: &EnumConstructor,
        span: Span,
    ) -> HirExpression {
        let args = constructor
            .arguments
            .iter()
            .map(|argument| {
                let value = self.lower_expression(&argument.expression, &Expected::free());
                let value = match (&argument.field_name, &argument.modifier) {
                    (None, Some(DeclModifier::Mut)) => value.adjusted(Adjustment::MutBorrow),
                    (None, Some(DeclModifier::Ref)) => {
                        self.lower_ref_value(value, argument.expression.span)
                    }
                    _ => value,
                };
                HirEnumConstructorArg {
                    field: argument.field_name.clone(),
                    value,
                }
            })
            .collect();

        HirExpression::new(
            HirExpressionKind::EnumConstructor(HirEnumConstructor {
                target: constructor.enum_access.target.clone(),
                case: constructor.enum_access.case.clone(),
                args,
            }),
            plain_type(constructor.enum_access.target.clone()),
            Ownership::UniqueOwned,
            span,
        )
    }

    fn lower_closure(
        &mut self,
        closure: &Closure,
        expected: &Expected,
        deref_params: bool,
        span: Span,
    ) -> HirExpression {
        let expected_closure = match &expected.ty {
            TypeElement::Closure(closure_ty) => Some(closure_ty.as_ref()),
            _ => None,
        };

        self.scopes.push();
        let parameters: Vec<HirClosureParam> = closure
            .parameters
            .iter()
            .enumerate()
            .map(|(i, parameter): (usize, &ClosureParameter)| {
                let ty = if parameter.ty.is_infer() {
                    expected_closure
                        .and_then(|closure_ty| closure_ty.parameters.get(i))
                        .cloned()
                        .unwrap_or_else(TypeElement::infer)
                } else {
                    parameter.ty.clone()
                };
                let ownership = if self.is_copy(&ty) && !ty.is_infer() {
                    Ownership::UniqueOwned
                } else {
                    Ownership::Borrowed
                };
                self.scopes.declare(Variable {
                    ident: parameter.ident.clone(),
                    modifier: DeclModifier::Let,
                    ty: ty.clone(),
                    ownership,
                });
                HirClosureParam {
                    ident: parameter.ident.clone(),
                    ty,
                    deref: deref_params,
                }
            })
            .collect();

        let body_expected = match expected_closure {
            Some(closure_ty)
                if !closure_ty.return_ty.is_infer() && !closure_ty.return_ty.is_void() =>
            {
                Expected::owned(closure_ty.return_ty.clone())
            }
            _ => Expected::free(),
        };
        let body = self.lower_block(&closure.block.body, &body_expected);
        self.scopes.pop();

        let ty = TypeElement::Closure(Box::new(ClosureTypeItem {
            parameters: parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            return_ty: body.ty.clone(),
            span: Span::default(),
        }));

        HirExpression::new(
            HirExpressionKind::Closure(Box::new(HirClosure { parameters, body })),
            ty,
            Ownership::UniqueOwned,
            span,
        )
    }
}

// ----------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------

fn plain_type(ident: TypeIdent) -> TypeElement {
    TypeElement::Plain(BasicTypeItem {
        ident,
        span: Span::default(),
    })
}

fn closure_argument(argument: &FunctionCallArg) -> Option<&Closure> {
    match &argument.expression.kind {
        ExpressionKind::Closure(closure) => Some(closure),
        _ => None,
    }
}

/// Unifies the types of two branches of a conditional
fn unify_types(a: &TypeElement, b: &TypeElement) -> Option<TypeElement> {
    match (a, b) {
        (TypeElement::Never(_), other) | (other, TypeElement::Never(_)) => Some(other.clone()),
        (TypeElement::Infer(_), other) | (other, TypeElement::Infer(_)) => Some(other.clone()),
        (a, b) if a.is_number() => Some(b.clone()),
        (a, b) if b.is_number() => Some(a.clone()),
        (a, b) if types_compatible(a, b) => Some(a.clone()),
        (TypeElement::Optional(optional), other) if types_compatible(&optional.inner, other) => {
            Some(other.clone())
        }
        _ => None,
    }
}

/// The element type produced by each iteration when a `for` loop is used as
/// an expression with the given expected type
fn iteration_type(_fn_return: &TypeElement, expected: &TypeElement) -> TypeElement {
    match expected {
        TypeElement::Array(array) => array.elements.clone(),
        TypeElement::Optional(optional) => iteration_type(_fn_return, &optional.inner),
        TypeElement::Result(result) => iteration_type(_fn_return, &result.success),
        TypeElement::Never(never) => TypeElement::Never(never.clone()),
        TypeElement::Infer(_) => TypeElement::infer(),
        TypeElement::Void(_) => TypeElement::void(),
        _ => TypeElement::infer(),
    }
}

/// Infer the most appropriate type for a number literal
fn infer_number_type(value: &str) -> TypeElement {
    if value.contains('.') || value.contains('e') || value.contains('E') {
        return if value.ends_with("f32") {
            plain_type(TypeIdent::new("Float"))
        } else {
            // Floats without explicit suffix use the __Number intrinsic so
            // rustc decides the width
            plain_type(TypeIdent::new("__Number"))
        };
    }

    if let Some(type_name) = extract_type_suffix(value) {
        return plain_type(TypeIdent::new(type_name));
    }

    // Integer literals without suffix resolve through rustc's inference
    plain_type(TypeIdent::new("__Number"))
}

/// Extract type suffix from number literal (e.g., "42i32" -> Some("I32"))
fn extract_type_suffix(value: &str) -> Option<&'static str> {
    if value.ends_with("i8") {
        Some("I8")
    } else if value.ends_with("i16") {
        Some("I16")
    } else if value.ends_with("i32") {
        Some("I32")
    } else if value.ends_with("i64") {
        Some("I64")
    } else if value.ends_with("i128") {
        Some("I128")
    } else if value.ends_with("isize") {
        Some("ISize")
    } else if value.ends_with("u8") {
        Some("U8")
    } else if value.ends_with("u16") {
        Some("U16")
    } else if value.ends_with("u32") {
        Some("U32")
    } else if value.ends_with("u64") {
        Some("U64")
    } else if value.ends_with("u128") {
        Some("U128")
    } else if value.ends_with("usize") {
        Some("USize")
    } else if value.ends_with("f32") {
        Some("Float")
    } else if value.ends_with("f64") {
        Some("Double")
    } else {
        None
    }
}

fn are_compatible_numeric_types(a: &TypeIdent, b: &TypeIdent) -> bool {
    let integer_types = [
        "I8", "I16", "I32", "I64", "I128", "ISize", "Int", "U8", "U16", "U32", "U64", "U128",
        "USize", "UInt",
    ];
    let float_types = ["Float", "Double"];

    let a = a.as_str();
    let b = b.as_str();

    (integer_types.contains(&a) && integer_types.contains(&b))
        || (float_types.contains(&a) && float_types.contains(&b))
        || a.starts_with("__")
        || b.starts_with("__")
}

fn modifier_name(modifier: DeclModifier) -> &'static str {
    match modifier {
        DeclModifier::Let => "let",
        DeclModifier::Mut => "mut",
        DeclModifier::Ref => "ref",
    }
}
