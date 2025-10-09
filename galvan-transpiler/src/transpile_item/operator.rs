use galvan_ast::{
    ArithmeticOperator, CollectionOperator, ComparisonOperator, CustomInfix, InfixExpression,
    InfixOperation, LogicalOperator, Ownership, RangeOperator, TypeElement,
};
use galvan_resolver::Scope;

use crate::error::ErrorCollector;
use crate::macros::impl_transpile_variants;
use crate::type_inference::InferType;
use crate::Transpile;
use crate::{context::Context, transpile};

impl_transpile_variants!(InfixExpression; Arithmetic, Logical, Collection, Range, Comparison, Custom, Member);

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
            ComparisonOperator::Equal => transpile!(ctx, scope, errors, "({}).eq(&{})", lhs, rhs),
            ComparisonOperator::NotEqual => {
                transpile!(ctx, scope, errors, "({}).ne(&{})", lhs, rhs)
            }
            ComparisonOperator::Less => transpile!(ctx, scope, errors, "({}).lt(&{})", lhs, rhs),
            ComparisonOperator::LessEqual => {
                transpile!(ctx, scope, errors, "({}).le(&{})", lhs, rhs)
            }
            ComparisonOperator::Greater => transpile!(ctx, scope, errors, "({}).gt(&{})", lhs, rhs),
            ComparisonOperator::GreaterEqual => {
                transpile!(ctx, scope, errors, "({}).ge(&{})", lhs, rhs)
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
                let lhs_type = lhs.infer_type(scope, errors);
                let rhs_type = rhs.infer_type(scope, errors);

                match &lhs_type {
                    TypeElement::Array(array_type) => {
                        if rhs_type == array_type.elements {
                            transpile!(
                                ctx,
                                scope,
                                errors,
                                "{{ let mut temp = ({}).to_owned(); temp.push({}); temp }}",
                                lhs,
                                rhs
                            )
                        } else {
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
                    TypeElement::Set(set_type) => {
                        if rhs_type == set_type.elements {
                            transpile!(
                                ctx,
                                scope,
                                errors,
                                "{{ let mut temp = ({}).to_owned(); temp.insert({}); temp }}",
                                lhs,
                                rhs
                            )
                        } else {
                            transpile!(
                                ctx,
                                scope,
                                errors,
                                "({}).union(&{}).cloned().collect::<::std::collections::HashSet<_>>().to_owned()",
                                lhs,
                                rhs
                            )
                        }
                    }
                    TypeElement::Plain(basic_type) if basic_type.ident.as_str() == "String" => {
                        if let TypeElement::Plain(rhs_basic) = &rhs_type {
                            if rhs_basic.ident.as_str() == "Char" {
                                transpile!(
                                    ctx,
                                    scope,
                                    errors,
                                    "{{ let mut temp = ({}).to_owned(); temp.push({}); temp }}",
                                    lhs,
                                    rhs
                                )
                            } else if rhs_basic.ident.as_str() == "String" {
                                transpile!(
                                    ctx,
                                    scope,
                                    errors,
                                    "format!(\"{{}}{{}}\" , {}, {})",
                                    lhs,
                                    rhs
                                )
                            } else {
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
