use crate::macros::{impl_transpile_variants, transpile};
use crate::Transpile;
use galvan_ast::{Assignment, AssignmentOperator, AssignmentTarget};
use galvan_resolver::LookupContext;

impl_transpile_variants!(AssignmentTarget; Ident, MemberFieldAccess);

impl Transpile for Assignment {
    fn transpile(&self, lookup: &LookupContext) -> String {
        // TODO: Use scope to determine if variable is &mut or owned, dereference is only needed for &mut
        let deref = "*";
        let Self {
            target,
            operator,
            expression: exp,
        } = self;
        match operator {
            AssignmentOperator::Assign => {
                transpile!(lookup, "{deref}{} = {}", target, exp)
            }
            AssignmentOperator::AddAssign => {
                transpile!(lookup, "{deref}{} += {}", target, exp)
            }
            AssignmentOperator::SubAssign => {
                transpile!(lookup, "{deref}{} -= {}", target, exp)
            }
            AssignmentOperator::MulAssign => {
                transpile!(lookup, "{deref}{} *= {}", target, exp)
            }
            AssignmentOperator::DivAssign => {
                transpile!(lookup, "{deref}{} /= {}", target, exp)
            }
            AssignmentOperator::RemAssign => {
                transpile!(lookup, "{deref}{} %= {}", target, exp)
            }
            AssignmentOperator::PowAssign => {
                transpile!(lookup, "{deref}{} = {}.pow({})", target, target, exp)
            }
        }
    }
}
