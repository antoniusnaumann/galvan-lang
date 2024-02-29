use crate::context::Context;
use crate::macros::{impl_transpile_variants, transpile};
use crate::Transpile;
use galvan_ast::{Assignment, AssignmentOperator, Ownership, Expression};
use galvan_resolver::Scope;

impl Transpile for Assignment {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        // TODO: Use scope to determine if variable is &mut or owned, dereference is only needed for &mut
        let Self {
            target,
            operator,
            expression: exp,
        } = self;

        let prefix = match target {
            Expression::Ident(ident) => scope.get_variable(ident).map_or("", |var| match var
                .ownership
            {
                Ownership::Owned => "",
                Ownership::Borrowed => todo!("Error: Cannot assign to borrowed variable"),
                Ownership::MutBorrowed => "*",
                Ownership::Copy => "",
                Ownership::Ref => todo!("Handle assignment to ref variable"),
            }),
            _ => "",
        };

        match operator {
            AssignmentOperator::Assign => {
                transpile!(ctx, scope, "{prefix}{} = {}", target, exp)
            }
            AssignmentOperator::AddAssign => {
                transpile!(ctx, scope, "{prefix}{} += {}", target, exp)
            }
            AssignmentOperator::SubAssign => {
                transpile!(ctx, scope, "{prefix}{} -= {}", target, exp)
            }
            AssignmentOperator::MulAssign => {
                transpile!(ctx, scope, "{prefix}{} *= {}", target, exp)
            }
            AssignmentOperator::DivAssign => {
                transpile!(ctx, scope, "{prefix}{} /= {}", target, exp)
            }
            AssignmentOperator::RemAssign => {
                transpile!(ctx, scope, "{prefix}{} %= {}", target, exp)
            }
            AssignmentOperator::PowAssign => {
                transpile!(ctx, scope, "{prefix}{} = {}.pow({})", target, target, exp)
            }
        }
    }
}
