use crate::cast::cast;
use crate::error::ErrorCollector;
use crate::macros::transpile;
use crate::Transpile;
use crate::{context::Context, type_inference::InferType};
use galvan_ast::{Assignment, AssignmentOperator, ExpressionKind, Ownership};
use galvan_resolver::Scope;

impl Transpile for Assignment {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // TODO: Use scope to determine if variable is &mut or owned, dereference is only needed for &mut
        let Self {
            target,
            operator,
            expression: exp,
            span: _span,
        } = self;

        let target_ty = target.infer_type(scope, errors);
        let ownership = target.infer_owned(ctx, scope, errors);
        let mut scope = Scope::child(scope).returns(target_ty, ownership);
        let scope = &mut scope;

        let prefix = match &target.kind {
            ExpressionKind::Ident(ident) => {
                scope
                    .get_variable(ident)
                    .map_or("", |var| match var.ownership {
                        Ownership::SharedOwned => "",
                        Ownership::Borrowed => {
                            // TODO: Add proper error handling for borrowed variable assignment
                            ""
                        },
                        Ownership::MutBorrowed => "*",
                        Ownership::UniqueOwned => "",
                        Ownership::Ref => todo!("Handle assignment to ref variable"),
                    })
            }
            _ => "",
        };

        let exp = cast(exp, &scope.return_type.clone(), ownership, ctx, scope, errors);

        match operator {
            AssignmentOperator::Assign => {
                transpile!(ctx, scope, errors, "{prefix}{} = {}", target, exp)
            }
            AssignmentOperator::AddAssign => {
                transpile!(ctx, scope, errors, "{prefix}{} += {}", target, exp)
            }
            AssignmentOperator::SubAssign => {
                transpile!(ctx, scope, errors, "{prefix}{} -= {}", target, exp)
            }
            AssignmentOperator::MulAssign => {
                transpile!(ctx, scope, errors, "{prefix}{} *= {}", target, exp)
            }
            AssignmentOperator::DivAssign => {
                transpile!(ctx, scope, errors, "{prefix}{} /= {}", target, exp)
            }
            AssignmentOperator::RemAssign => {
                transpile!(ctx, scope, errors, "{prefix}{} %= {}", target, exp)
            }
            AssignmentOperator::PowAssign => {
                transpile!(ctx, scope, errors, "{prefix}{} = {}.pow({})", target, target, exp)
            }
        }
    }
}
