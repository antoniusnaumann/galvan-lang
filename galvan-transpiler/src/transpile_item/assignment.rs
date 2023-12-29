use crate::macros::transpile;
use crate::Transpile;
use galvan_ast::{Assignment, AssignmentOperator};
use galvan_resolver::LookupContext;

impl Transpile for Assignment {
    fn transpile(&self, lookup: &LookupContext) -> String {
        // TODO: Use scope to determine if variable is &mut or owned, dereference is only needed for &mut
        let deref = "*";
        let Self {
            identifier: ident,
            operator,
            expression: exp,
        } = self;
        match operator {
            AssignmentOperator::Assign => {
                transpile!(lookup, "{deref}{} = {}", ident, exp)
            }
            AssignmentOperator::AddAssign => {
                transpile!(lookup, "{deref}{} += {}", ident, exp)
            }
            AssignmentOperator::SubAssign => {
                transpile!(lookup, "{deref}{} -= {}", ident, exp)
            }
            AssignmentOperator::MulAssign => {
                transpile!(lookup, "{deref}{} *= {}", ident, exp)
            }
            AssignmentOperator::DivAssign => {
                transpile!(lookup, "{deref}{} /= {}", ident, exp)
            }
            AssignmentOperator::RemAssign => {
                transpile!(lookup, "{deref}{} %= {}", ident, exp)
            }
            AssignmentOperator::PowAssign => {
                transpile!(lookup, "{deref}{} = {}.pow({})", ident, ident, exp)
            }
        }
    }
}
