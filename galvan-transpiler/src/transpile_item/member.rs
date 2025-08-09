use crate::cast::cast;
use crate::context::Context;
use crate::error::ErrorCollector;
use crate::macros::transpile;
use crate::type_inference::InferType;
use crate::Transpile;
use galvan_ast::{ConstructorCall, ConstructorCallArg, InfixOperation, MemberOperator, Ownership, TypeDecl};
use galvan_resolver::{Lookup, Scope};

impl Transpile for InfixOperation<MemberOperator> {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let Self { lhs, operator, rhs } = self;
        match operator {
            MemberOperator::Dot => transpile!(ctx, scope, errors, "{}.{}", lhs, rhs),
            MemberOperator::SafeCall => {
                transpile!(ctx, scope, errors, "{}.map(|__elem__| {{ __elem__.{} }})", lhs, rhs)
            }
        }
    }
}

impl Transpile for ConstructorCall {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // Look up the struct type to get field types for casting
        let type_decl = ctx.lookup.resolve_type(&self.identifier);
        
        let arguments_str = if let Some(type_item) = type_decl {
            if let TypeDecl::Struct(struct_decl) = &type_item.item {
                // Create a map of field name to field type for quick lookup
                let field_types: std::collections::HashMap<_, _> = struct_decl
                    .members
                    .iter()
                    .map(|member| (member.ident.as_str(), &member.r#type))
                    .collect();
                
                self.arguments
                    .iter()
                    .map(|arg| {
                        // Get the expected field type
                        if let Some(expected_type) = field_types.get(arg.ident.as_str()) {
                            // Use cast to convert the argument to the expected type
                            let cast_expr = cast(&arg.expression, expected_type, Ownership::UniqueOwned, ctx, scope, errors);
                            format!("{}: {}", arg.ident.transpile(ctx, scope, errors), cast_expr)
                        } else {
                            // Fallback to original behavior if field not found
                            arg.transpile(ctx, scope, errors)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                // Not a struct, use original behavior
                self.arguments
                    .iter()
                    .map(|arg| arg.transpile(ctx, scope, errors))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        } else {
            // Type not found, use original behavior
            self.arguments
                .iter()
                .map(|arg| arg.transpile(ctx, scope, errors))
                .collect::<Vec<_>>()
                .join(", ")
        };
        
        format!("{} {{ {} }}", self.identifier.transpile(ctx, scope, errors), arguments_str)
    }
}

impl crate::Transpile for ConstructorCallArg {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let postfix = match self.expression.infer_owned(ctx, scope) {
            Ownership::SharedOwned => ".clone()",
            Ownership::UniqueOwned => "",
            Ownership::Borrowed | Ownership::MutBorrowed | Ownership::Ref => ".to_owned()",
        };
        transpile!(ctx, scope, errors, "{}: {}{postfix}", self.ident, self.expression)
    }
}
