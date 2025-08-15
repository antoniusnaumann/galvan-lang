use galvan_ast::{
    ArithmeticOperator, CollectionOperator, ComparisonOperator, CustomInfix, InfixExpression,
    InfixOperation, LogicalOperator, Ownership, RangeOperator, TypeElement, UnwrapOperator,
};
use galvan_resolver::Scope;

use crate::error::ErrorCollector;
use crate::macros::impl_transpile_variants;
use crate::type_inference::InferType;
use crate::Transpile;
use crate::{context::Context, transpile};

impl_transpile_variants!(InfixExpression; Arithmetic, Logical, Collection, Range, Comparison, Unwrap, Custom, Member);

impl Transpile for InfixOperation<LogicalOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let Self { lhs, operator, rhs } = self;
        match operator {
            LogicalOperator::And => transpile!(ctx, scope, errors, "{} && {}", lhs, rhs),
            LogicalOperator::Or => transpile!(ctx, scope, errors, "{} || {}", lhs, rhs),
            LogicalOperator::Xor => todo!("Correctly handle xor chains"),
        }
    }
}

impl Transpile for InfixOperation<ComparisonOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let Self { lhs, operator, rhs } = self;

        match operator {
            ComparisonOperator::Equal => transpile!(ctx, scope, errors, "{} == {}", lhs, rhs),
            ComparisonOperator::NotEqual => {
                transpile!(ctx, scope, errors, "{} != {}", lhs, rhs)
            }
            ComparisonOperator::Less => transpile!(ctx, scope, errors, "{} < {}", lhs, rhs),
            ComparisonOperator::LessEqual => {
                transpile!(ctx, scope, errors, "{} <= {}", lhs, rhs)
            }
            ComparisonOperator::Greater => transpile!(ctx, scope, errors, "{} > {}", lhs, rhs),
            ComparisonOperator::GreaterEqual => {
                transpile!(ctx, scope, errors, "{} >= {}", lhs, rhs)
            }
            ComparisonOperator::Identical => {
                transpile!(
                    ctx,
                    scope,
                    errors,
                    "::std::sync::Arc::ptr_eq({}, {})",
                    lhs,
                    rhs
                )
            }
            ComparisonOperator::NotIdentical => {
                transpile!(
                    ctx,
                    scope,
                    errors,
                    "!::std::sync::Arc::ptr_eq({}, {})",
                    lhs,
                    rhs
                )
            }
        }
    }
}

impl Transpile for InfixOperation<CollectionOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let Self { lhs, operator, rhs } = self;

        match operator {
            CollectionOperator::Concat => {
                // Determine if RHS is a collection (use concat) or element (use single-element append)
                // Default to concat behavior to be consistent with ++= operator
                let lhs_type = lhs.infer_type(scope, errors);
                let rhs_type = rhs.infer_type(scope, errors);

                // Check if LHS is an array/vector type
                match &lhs_type {
                    TypeElement::Array(array_type) => {
                        // If RHS type exactly matches the element type, append as single element
                        // Otherwise, default to concat (consistent with ++= operator behavior)
                        if rhs_type == array_type.elements {
                            // Single element append: create a new vector with the element added
                            transpile!(
                                ctx,
                                scope,
                                errors,
                                "{{ let mut temp = ({}).to_owned(); temp.push({}); temp }}",
                                lhs,
                                rhs
                            )
                        } else {
                            // Default to concat behavior (consistent with ++= operator)
                            transpile!(
                                ctx,
                                scope,
                                errors,
                                "[({}).to_owned(), ({}).to_owned()].concat()",
                                lhs,
                                rhs
                            )
                        }
                    }
                    TypeElement::Plain(basic_type) if basic_type.ident.as_str() == "String" => {
                        // Check if RHS is a char
                        if let TypeElement::Plain(rhs_basic) = &rhs_type {
                            if rhs_basic.ident.as_str() == "Char" {
                                // String + char: use push method
                                transpile!(
                                    ctx,
                                    scope,
                                    errors,
                                    "{{ let mut temp = ({}).to_owned(); temp.push({}); temp }}",
                                    lhs,
                                    rhs
                                )
                            } else if rhs_basic.ident.as_str() == "String" {
                                // String + String: existing logic
                                transpile!(
                                    ctx,
                                    scope,
                                    errors,
                                    "format!(\"{{}}{{}}\" , {}, {})",
                                    lhs,
                                    rhs
                                )
                            } else {
                                // String + other: existing conversion logic
                                transpile!(
                                    ctx,
                                    scope,
                                    errors,
                                    "format!(\"{{}}{{}}\" , {}, {})",
                                    lhs,
                                    rhs
                                )
                            }
                        } else {
                            // String + complex type: existing default logic
                            transpile!(
                                ctx,
                                scope,
                                errors,
                                "format!(\"{{}}{{}}\" , {}, {})",
                                lhs,
                                rhs
                            )
                        }
                    }
                    _ => {
                        // LHS is not an array or string, default to concat behavior
                        // (consistent with ++= operator which assumes collections)
                        transpile!(
                            ctx,
                            scope,
                            errors,
                            "[({}).to_owned(), ({}).to_owned()].concat()",
                            lhs,
                            rhs
                        )
                    }
                }
            }
            CollectionOperator::Remove => todo!("Implement remove operator"),
            CollectionOperator::Contains => {
                transpile!(ctx, scope, errors, "({}).contains(&({}))", rhs, lhs)
            }
        }
    }
}

impl Transpile for InfixOperation<ArithmeticOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let Self { lhs, operator, rhs } = self;

        match operator {
            ArithmeticOperator::Add => transpile!(ctx, scope, errors, "{} + {}", lhs, rhs),
            ArithmeticOperator::Sub => transpile!(ctx, scope, errors, "{} - {}", lhs, rhs),
            ArithmeticOperator::Mul => transpile!(ctx, scope, errors, "{} * {}", lhs, rhs),
            ArithmeticOperator::Div => transpile!(ctx, scope, errors, "{} / {}", lhs, rhs),
            ArithmeticOperator::Rem => transpile!(ctx, scope, errors, "{} % {}", lhs, rhs),
            ArithmeticOperator::Exp => transpile!(ctx, scope, errors, "{}.pow({})", lhs, rhs),
        }
    }
}

impl Transpile for InfixOperation<UnwrapOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let Self {
            lhs,
            operator: _,
            rhs,
        } = self;

        // Check ownership of both sides to determine if we need to clone
        let lhs_ownership = lhs.infer_owned(ctx, scope, errors);
        let rhs_ownership = rhs.infer_owned(ctx, scope, errors);

        // For the left-hand side (receiver), we need to clone if it's borrowed to avoid move issues
        let lhs_clone_suffix = match lhs_ownership {
            Ownership::SharedOwned => ".clone()",
            Ownership::Borrowed | Ownership::MutBorrowed => ".clone()",
            Ownership::UniqueOwned | Ownership::Ref => "",
        };

        // For the right-hand side, we need to clone to avoid move issues when captured in closure
        let rhs_clone_suffix = match rhs_ownership {
            Ownership::SharedOwned => ".clone()",
            Ownership::Borrowed | Ownership::MutBorrowed => ".to_owned()",
            Ownership::UniqueOwned | Ownership::Ref => "",
        };

        // TODO: this should be a match expression instead to allow return from the left arm and so on
        transpile!(
            ctx,
            scope,
            errors,
            "({}{}).unwrap_or_else(|| {}{})",
            lhs,
            lhs_clone_suffix,
            rhs,
            rhs_clone_suffix
        )
    }
}

impl Transpile for InfixOperation<CustomInfix> {
    fn transpile(
        &self,
        _ctx: &Context,
        _scope: &mut Scope,
        _errors: &mut ErrorCollector,
    ) -> String {
        todo!("Implement custom infix operator!")
    }
}

impl Transpile for InfixOperation<RangeOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let Self { lhs, operator, rhs } = self;

        match operator {
            RangeOperator::Inclusive => {
                transpile!(ctx, scope, errors, "{}..=({})", lhs, rhs)
            }
            RangeOperator::Exclusive => {
                transpile!(ctx, scope, errors, "{}..({})", lhs, rhs)
            }
            RangeOperator::Tolerance => {
                // center ± tolerance => (center - tolerance)..=(center + tolerance)
                transpile!(
                    ctx,
                    scope,
                    errors,
                    "({} - {})..=({} + {})",
                    lhs,
                    rhs,
                    lhs,
                    rhs
                )
            }
            RangeOperator::Interval => {
                // start ..+ interval => start..(start + interval)
                transpile!(ctx, scope, errors, "{}..({} + {})", lhs, lhs, rhs)
            }
        }
    }
}
