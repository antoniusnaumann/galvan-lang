use galvan_ast::{
    ArithmeticOperator, CollectionOperator, ComparisonOperator, CustomInfix, InfixExpression,
    InfixOperation, LogicalOperator, UnwrapOperator,
};
use galvan_resolver::Scope;

use crate::error::ErrorCollector;
use crate::macros::impl_transpile_variants;
use crate::{context::Context, transpile};
use crate::Transpile;

impl_transpile_variants!(InfixExpression; Arithmetic, Logical, Collection, Comparison, Unwrap, Custom, Member);

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
                transpile!(ctx, scope, errors, "::std::sync::Arc::ptr_eq({}, {})", lhs, rhs)
            }
            ComparisonOperator::NotIdentical => {
                transpile!(ctx, scope, errors, "!::std::sync::Arc::ptr_eq({}, {})", lhs, rhs)
            }
        }
    }
}

impl Transpile for InfixOperation<CollectionOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let Self { lhs, operator, rhs } = self;

        match operator {
            CollectionOperator::Concat => {
                // TODO: Check if underlying expression is already owned or copy
                transpile!(
                    ctx,
                    scope,
                    errors,
                    "[({}).to_owned(), ({}).to_owned()].concat()",
                    lhs,
                    rhs
                )
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

        // TODO: this should be a match expression instead to allow return from the left arm and so on
        transpile!(ctx, scope, errors, "({}).unwrap_or_else(|| {})", lhs, rhs)
    }
}

impl Transpile for InfixOperation<CustomInfix> {
    fn transpile(&self, _ctx: &Context, _scope: &mut Scope, _errors: &mut ErrorCollector) -> String {
        todo!("Implement custom infix operator!")
    }
}
