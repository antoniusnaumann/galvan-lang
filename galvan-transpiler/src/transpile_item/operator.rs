use crate::macros::impl_transpile_variants;
use crate::Transpile;
use crate::{context::Context, transpile};
use galvan_ast::{
    ArithmeticOperator, CollectionOperator, ComparisonOperator, CustomInfix, InfixExpression, InfixOperation, LogicalOperator
};
use galvan_resolver::Scope;

impl_transpile_variants!(InfixExpression; Arithmetic, Logical, Collection, Comparison, Custom, Member);

impl Transpile for InfixOperation<LogicalOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self { lhs, operator, rhs, span: _span } = self;
        match operator {
            LogicalOperator::And => transpile!(ctx, scope, "{} && {}", lhs, rhs),
            LogicalOperator::Or => transpile!(ctx, scope, "{} || {}", lhs, rhs),
            LogicalOperator::Xor => todo!("Correctly handle xor chains"),
        }
    }
}

impl Transpile for InfixOperation<ComparisonOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self { lhs, operator, rhs, span: _span } = self;

        match operator {
            ComparisonOperator::Equal => transpile!(ctx, scope, "{} == {}", lhs, rhs),
            ComparisonOperator::NotEqual => {
                transpile!(ctx, scope, "{} != {}", lhs, rhs)
            }
            ComparisonOperator::Less => transpile!(ctx, scope, "{} < {}", lhs, rhs),
            ComparisonOperator::LessEqual => {
                transpile!(ctx, scope, "{} <= {}", lhs, rhs)
            }
            ComparisonOperator::Greater => transpile!(ctx, scope, "{} > {}", lhs, rhs),
            ComparisonOperator::GreaterEqual => {
                transpile!(ctx, scope, "{} >= {}", lhs, rhs)
            }
            ComparisonOperator::Identical => {
                transpile!(ctx, scope, "::std::sync::Arc::ptr_eq({}, {})", lhs, rhs)
            }
            ComparisonOperator::NotIdentical => {
                transpile!(ctx, scope, "!::std::sync::Arc::ptr_eq({}, {})", lhs, rhs)
            }
        }
    }
}

impl Transpile for InfixOperation<CollectionOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self { lhs, operator, rhs, span: _span } = self;

        match operator {
            CollectionOperator::Concat => {
                // TODO: Check if underlying expression is already owned or copy
                transpile!(
                    ctx,
                    scope,
                    "[({}).to_owned(), ({}).to_owned()].concat()",
                    lhs,
                    rhs
                )
            }
            CollectionOperator::Remove => todo!("Implement remove operator"),
            CollectionOperator::Contains => {
                transpile!(ctx, scope, "({}).contains(&({}))", rhs, lhs)
            }
        }
    }
}

impl Transpile for InfixOperation<ArithmeticOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self { lhs, operator, rhs, span: _span } = self;

        match operator {
            ArithmeticOperator::Add => transpile!(ctx, scope, "{} + {}", lhs, rhs),
            ArithmeticOperator::Sub => transpile!(ctx, scope, "{} - {}", lhs, rhs),
            ArithmeticOperator::Mul => transpile!(ctx, scope, "{} * {}", lhs, rhs),
            ArithmeticOperator::Div => transpile!(ctx, scope, "{} / {}", lhs, rhs),
            ArithmeticOperator::Rem => transpile!(ctx, scope, "{} % {}", lhs, rhs),
            ArithmeticOperator::Exp => transpile!(ctx, scope, "{}.pow({})", lhs, rhs),
        }
    }
}

impl Transpile for InfixOperation<CustomInfix> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        todo!("Implement custom infix operator!")
    }
}
