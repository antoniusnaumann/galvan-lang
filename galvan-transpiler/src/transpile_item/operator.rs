use crate::context::Context;
use crate::macros::{impl_transpile_variants, transpile};
use crate::Transpile;
use galvan_ast::{
    ArithmeticOperator, CollectionOperator, ComparisonOperator, InfixOperator, LogicalOperator,
    OperatorTree, OperatorTreeNode, SimpleExpression,
};
use galvan_resolver::Scope;

impl Transpile for OperatorTree {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let Self {
            left,
            operator,
            right,
        } = self;

        match operator {
            InfixOperator::Arithmetic(op) => transpile_arithmetic(ctx, scope, *op, left, right),
            InfixOperator::Collection(op) => {
                transpile_collection_operation(ctx, scope, *op, left, right)
            }
            InfixOperator::Comparison(op) => transpile_comparison(ctx, scope, *op, left, right),
            InfixOperator::Logical(op) => transpile_logical(ctx, scope, *op, left, right),
            InfixOperator::CustomInfix(op) => {
                todo!("Implement custom operator support")
            }
        }
    }
}

impl_transpile_variants!(OperatorTreeNode; Operation, SimpleExpression);
impl_transpile_variants!(SimpleExpression; MemberFieldAccess, MemberFunctionCall, SingleExpression);

fn transpile_logical(
    ctx: &Context,
    scope: &mut Scope,
    op: LogicalOperator,
    left: &OperatorTreeNode,
    right: &OperatorTreeNode,
) -> String {
    match op {
        LogicalOperator::And => transpile!(ctx, scope, "{} && {}", left, right),
        LogicalOperator::Or => transpile!(ctx, scope, "{} || {}", left, right),
        LogicalOperator::Xor => todo!("Correctly handle xor chains"),
    }
}

fn transpile_comparison(
    ctx: &Context,
    scope: &mut Scope,
    op: ComparisonOperator,
    left: &OperatorTreeNode,
    right: &OperatorTreeNode,
) -> String {
    match op {
        ComparisonOperator::Equal => transpile!(ctx, scope, "{} == {}", left, right),
        ComparisonOperator::NotEqual => {
            transpile!(ctx, scope, "{} != {}", left, right)
        }
        ComparisonOperator::Less => transpile!(ctx, scope, "{} < {}", left, right),
        ComparisonOperator::LessEqual => {
            transpile!(ctx, scope, "{} <= {}", left, right)
        }
        ComparisonOperator::Greater => transpile!(ctx, scope, "{} > {}", left, right),
        ComparisonOperator::GreaterEqual => {
            transpile!(ctx, scope, "{} >= {}", left, right)
        }
        ComparisonOperator::Identical => {
            transpile!(ctx, scope, "::std::sync::Arc::ptr_eq({}, {})", left, right)
        }
        ComparisonOperator::NotIdentical => {
            transpile!(ctx, scope, "!::std::sync::Arc::ptr_eq({}, {})", left, right)
        }
    }
}

fn transpile_collection_operation(
    ctx: &Context,
    scope: &mut Scope,
    op: CollectionOperator,
    left: &OperatorTreeNode,
    right: &OperatorTreeNode,
) -> String {
    match op {
        CollectionOperator::Concat => {
            // TODO: Check if underlying expression is already owned or copy
            transpile!(
                ctx,
                scope,
                "[({}).to_owned(), ({}).to_owned()].concat()",
                left,
                right
            )
        }
        CollectionOperator::Remove => todo!("Implement remove operator"),
        CollectionOperator::Contains => {
            transpile!(ctx, scope, "({}).contains(&({}))", right, left)
        }
    }
}

fn transpile_arithmetic(
    ctx: &Context,
    scope: &mut Scope,
    op: ArithmeticOperator,
    left: &OperatorTreeNode,
    right: &OperatorTreeNode,
) -> String {
    match op {
        ArithmeticOperator::Plus => transpile!(ctx, scope, "{} + {}", left, right),
        ArithmeticOperator::Minus => transpile!(ctx, scope, "{} - {}", left, right),
        ArithmeticOperator::Multiply => transpile!(ctx, scope, "{} * {}", left, right),
        ArithmeticOperator::Divide => transpile!(ctx, scope, "{} / {}", left, right),
        ArithmeticOperator::Remainder => transpile!(ctx, scope, "{} % {}", left, right),
        ArithmeticOperator::Power => transpile!(ctx, scope, "{}.pow({})", left, right),
    }
}
