use crate::context::Context;
use crate::macros::{impl_transpile_variants, transpile};
use crate::Transpile;
use galvan_ast::{Assignment, AssignmentOperator, AssignmentTarget};

impl_transpile_variants!(AssignmentTarget; Ident, MemberFieldAccess);

impl Transpile for Assignment {
    fn transpile(&self, ctx: &Context) -> String {
        // TODO: Use scope to determine if variable is &mut or owned, dereference is only needed for &mut
        let deref = match self.target {
            AssignmentTarget::Ident(_) => "*",
            AssignmentTarget::MemberFieldAccess(_) => "",
        };
        let Self {
            target,
            operator,
            expression: exp,
        } = self;
        match operator {
            AssignmentOperator::Assign => {
                transpile!(ctx, "{deref}{} = {}", target, exp)
            }
            AssignmentOperator::AddAssign => {
                transpile!(ctx, "{deref}{} += {}", target, exp)
            }
            AssignmentOperator::SubAssign => {
                transpile!(ctx, "{deref}{} -= {}", target, exp)
            }
            AssignmentOperator::MulAssign => {
                transpile!(ctx, "{deref}{} *= {}", target, exp)
            }
            AssignmentOperator::DivAssign => {
                transpile!(ctx, "{deref}{} /= {}", target, exp)
            }
            AssignmentOperator::RemAssign => {
                transpile!(ctx, "{deref}{} %= {}", target, exp)
            }
            AssignmentOperator::PowAssign => {
                transpile!(ctx, "{deref}{} = {}.pow({})", target, target, exp)
            }
        }
    }
}
