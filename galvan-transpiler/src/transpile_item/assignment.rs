use crate::context::Context;
use crate::macros::{impl_transpile_variants, transpile};
use crate::Transpile;
use galvan_ast::{Assignment, AssignmentOperator, AssignmentTarget};
use galvan_resolver::Scope;

impl_transpile_variants!(AssignmentTarget; Ident, MemberFieldAccess);

impl Transpile for Assignment {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        // TODO: Use scope to determine if variable is &mut or owned, dereference is only needed for &mut
        let Self {
            target,
            operator,
            expression: exp,
        } = self;
        match operator {
            AssignmentOperator::Assign => {
                transpile!(ctx, scope, "*({}.__mut()) = {}", target, exp)
            }
            AssignmentOperator::AddAssign => {
                transpile!(ctx, scope, "*({}.__mut()) += {}", target, exp)
            }
            AssignmentOperator::SubAssign => {
                transpile!(ctx, scope, "*({}.__mut()) -= {}", target, exp)
            }
            AssignmentOperator::MulAssign => {
                transpile!(ctx, scope, "*({}.__mut()) *= {}", target, exp)
            }
            AssignmentOperator::DivAssign => {
                transpile!(ctx, scope, "*({}.__mut()) /= {}", target, exp)
            }
            AssignmentOperator::RemAssign => {
                transpile!(ctx, scope, "*({}.__mut()) %= {}", target, exp)
            }
            AssignmentOperator::PowAssign => {
                transpile!(
                    ctx,
                    scope,
                    "*({}.__mut()) = {}.pow({})",
                    target,
                    target,
                    exp
                )
            }
        }
    }
}
