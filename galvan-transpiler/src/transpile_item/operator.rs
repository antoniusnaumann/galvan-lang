use crate::context::Context;
use crate::macros::{impl_transpile_variants, transpile};
use crate::Transpile;
use galvan_ast::{
    ArithmeticOperation, ArithmeticOperator, CollectionOperation, CollectionOperator,
    ComparisonOperation, ComparisonOperator, LogicalOperation, LogicalOperator,
};
use itertools::Itertools;

impl Transpile for LogicalOperation {
    fn transpile(&self, ctx: &Context) -> String {
        let base = self.base.transpile(ctx);
        let chain = self
            .chain
            .iter()
            .map(|(op, expr)| match op {
                LogicalOperator::And => transpile!(ctx, "&& {}", expr),
                LogicalOperator::Or => transpile!(ctx, "|| {}", expr),
                LogicalOperator::Xor => todo!("Correctly handle xor chains"),
            })
            .join(" ");

        base + " " + &chain
    }
}

impl Transpile for ComparisonOperation {
    fn transpile(&self, ctx: &Context) -> String {
        match self.operator {
            ComparisonOperator::Equal => transpile!(ctx, "{} == {}", self.left, self.right),
            ComparisonOperator::NotEqual => transpile!(ctx, "{} != {}", self.left, self.right),
            ComparisonOperator::Less => transpile!(ctx, "{} < {}", self.left, self.right),
            ComparisonOperator::LessEqual => transpile!(ctx, "{} <= {}", self.left, self.right),
            ComparisonOperator::Greater => transpile!(ctx, "{} > {}", self.left, self.right),
            ComparisonOperator::GreaterEqual => transpile!(ctx, "{} >= {}", self.left, self.right),
            ComparisonOperator::Identical => transpile!(
                ctx,
                "::std::sync::Arc::ptr_eq({}, {})",
                self.left,
                self.right
            ),
            ComparisonOperator::NotIdentical => transpile!(
                ctx,
                "!::std::sync::Arc::ptr_eq({}, {})",
                self.left,
                self.right
            ),
        }
    }
}

impl Transpile for CollectionOperation {
    fn transpile(&self, ctx: &Context) -> String {
        match self.operator {
            CollectionOperator::Concat => {
                // TODO: Check if underlying expression is already owned or copy
                transpile!(
                    ctx,
                    "[({}).to_owned(), ({}).to_owned()].concat()",
                    self.left,
                    self.right
                )
            }
            CollectionOperator::Remove => todo!("Implement remove operator"),
            CollectionOperator::Contains => {
                transpile!(ctx, "({}).contains(&({}))", self.right, self.left)
            }
        }
    }
}

impl Transpile for ArithmeticOperation {
    fn transpile(&self, ctx: &Context) -> String {
        // As pow binds stronger than the other operators and
        // Rust already handles operator precedence, no pratt parsing is needed here
        let base = self.base.transpile(ctx);
        let chain = self
            .chain
            .iter()
            .map(|(op, expr)| match op {
                ArithmeticOperator::Plus => transpile!(ctx, "+ {}", expr),
                ArithmeticOperator::Minus => transpile!(ctx, "- {}", expr),
                ArithmeticOperator::Multiply => transpile!(ctx, "* {}", expr),
                ArithmeticOperator::Divide => transpile!(ctx, "/ {}", expr),
                ArithmeticOperator::Remainder => transpile!(ctx, "% {}", expr),
                ArithmeticOperator::Power => transpile!(ctx, ".pow({})", expr),
            })
            .join(" ");

        base + " " + &chain
    }
}
