use crate::cast::cast;
use crate::error::ErrorCollector;
use crate::macros::transpile;
use crate::type_inference::InferType;
use crate::Transpile;
use crate::context::Context;
use galvan_ast::{Assignment, AssignmentOperator, ExpressionKind, Ownership, TypeElement};
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
            AssignmentOperator::ConcatAssign => {
                // Determine if RHS is a collection (use extend) or element (use push)
                // Default to extend behavior to be consistent with ++ operator
                let target_type = target.infer_type(scope, errors);
                let rhs_type = self.expression.infer_type(scope, errors);
                
                // Check if target is an array/vector type
                match &target_type {
                    TypeElement::Array(array_type) => {
                        // If RHS type exactly matches the element type, use push
                        // Otherwise, default to extend (consistent with ++ operator behavior)
                        if rhs_type == array_type.elements {
                            transpile!(ctx, scope, errors, "{prefix}{}.push({})", target, exp)
                        } else {
                            // Default to extend behavior (consistent with ++ operator)
                            transpile!(ctx, scope, errors, "{prefix}{}.extend({})", target, exp)
                        }
                    }
                    TypeElement::Plain(basic_type) if basic_type.ident.as_str() == "String" => {
                        // String concatenation: append to existing string
                        // Check if RHS is also a string type or other
                        if let TypeElement::Plain(rhs_basic) = &rhs_type {
                            if rhs_basic.ident.as_str() == "String" {
                                // String + String: use push_str
                                transpile!(ctx, scope, errors, "{prefix}{}.push_str(&{})", target, exp)
                            } else {
                                // String + other (likely char): use push_str with conversion
                                transpile!(ctx, scope, errors, "{prefix}{}.push_str(&{}.to_string())", target, exp)
                            }
                        } else {
                            // Default to push_str with string conversion for complex types
                            transpile!(ctx, scope, errors, "{prefix}{}.push_str(&{}.to_string())", target, exp)
                        }
                    }
                    _ => {
                        // Target is not an array or string, default to extend behavior 
                        // (consistent with ++ operator which assumes collections)
                        transpile!(ctx, scope, errors, "{prefix}{}.extend({})", target, exp)
                    }
                }
            }
        }
    }
}
