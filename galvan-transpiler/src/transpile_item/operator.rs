use crate::context::Context;
use crate::macros::transpile;
use crate::Transpile;
use galvan_ast::{ArithmeticOperator, CollectionOperator, ComparisonOperator, LogicalOperator};
use galvan_resolver::Scope;
use itertools::Itertools;

impl Transpile for LogicalOperation {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let base = self.base.transpile(ctx, scope);
        let chain = self
            .chain
            .iter()
            .map(|(op, expr)| match op {
                LogicalOperator::And => transpile!(ctx, scope, "&& {}", expr),
                LogicalOperator::Or => transpile!(ctx, scope, "|| {}", expr),
                LogicalOperator::Xor => todo!("Correctly handle xor chains"),
            })
            .join(" ");

        base + " " + &chain
    }
}

impl Transpile for ComparisonOperation {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        match self.operator {
            ComparisonOperator::Equal => transpile!(ctx, scope, "{} == {}", self.left, self.right),
            ComparisonOperator::NotEqual => {
                transpile!(ctx, scope, "{} != {}", self.left, self.right)
            }
            ComparisonOperator::Less => transpile!(ctx, scope, "{} < {}", self.left, self.right),
            ComparisonOperator::LessEqual => {
                transpile!(ctx, scope, "{} <= {}", self.left, self.right)
            }
            ComparisonOperator::Greater => transpile!(ctx, scope, "{} > {}", self.left, self.right),
            ComparisonOperator::GreaterEqual => {
                transpile!(ctx, scope, "{} >= {}", self.left, self.right)
            }
            ComparisonOperator::Identical => transpile!(
                ctx,
                scope,
                "::std::sync::Arc::ptr_eq({}, {})",
                self.left,
                self.right
            ),
            ComparisonOperator::NotIdentical => transpile!(
                ctx,
                scope,
                "!::std::sync::Arc::ptr_eq({}, {})",
                self.left,
                self.right
            ),
        }
    }
}

impl Transpile for CollectionOperation {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        match self.operator {
            CollectionOperator::Concat => {
                // TODO: Check if underlying expression is already owned or copy
                transpile!(
                    ctx,
                    scope,
                    "[({}).to_owned(), ({}).to_owned()].concat()",
                    self.left,
                    self.right
                )
            }
            CollectionOperator::Remove => todo!("Implement remove operator"),
            CollectionOperator::Contains => {
                transpile!(ctx, scope, "({}).contains(&({}))", self.right, self.left)
            }
        }
    }
}

impl Transpile for ArithmeticOperation {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        // As pow binds stronger than the other operators and
        // Rust already handles operator precedence, no pratt parsing is needed here
        let base = self.base.transpile(ctx, scope);
        let chain = self
            .chain
            .iter()
            .map(|(op, expr)| match op {
                ArithmeticOperator::Plus => transpile!(ctx, scope, "+ {}", expr),
                ArithmeticOperator::Minus => transpile!(ctx, scope, "- {}", expr),
                ArithmeticOperator::Multiply => transpile!(ctx, scope, "* {}", expr),
                ArithmeticOperator::Divide => transpile!(ctx, scope, "/ {}", expr),
                ArithmeticOperator::Remainder => transpile!(ctx, scope, "% {}", expr),
                ArithmeticOperator::Power => transpile!(ctx, scope, ".pow({})", expr),
            })
            .join(" ");

        base + " " + &chain
    }
}
