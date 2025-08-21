use crate::cast::cast;
use crate::context::Context;
use crate::error::ErrorCollector;
use crate::macros::transpile;
use crate::type_inference::InferType;
use crate::{TranspilerError, Transpile};
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
                
                // Create a map of provided arguments by field name
                let provided_args: std::collections::HashMap<_, _> = self.arguments
                    .iter()
                    .map(|arg| (arg.ident.as_str(), arg))
                    .collect();
                
                // Build the final argument list including defaults
                let mut final_args = Vec::new();
                
                for member in &struct_decl.members {
                    let field_name = member.ident.as_str();
                    
                    if let Some(arg) = provided_args.get(field_name) {
                        // Field is explicitly provided in constructor call
                        if let Some(expected_type) = field_types.get(field_name) {
                            let cast_expr = cast(&arg.expression, expected_type, Ownership::UniqueOwned, ctx, scope, errors);
                            final_args.push(format!("{}: {}", arg.ident.transpile(ctx, scope, errors), cast_expr));
                        } else {
                            final_args.push(arg.transpile(ctx, scope, errors));
                        }
                    } else if let Some(ref default_value) = member.default_value {
                        // Field is missing but has a default value - inline the default expression
                        let default_transpiled = default_value.transpile(ctx, scope, errors);
                        final_args.push(format!("{}: {}", member.ident.transpile(ctx, scope, errors), default_transpiled));
                    } else {
                        // Field is missing and has no default - this should be a compilation error
                        let required_fields = struct_decl.members.len();
                        let provided_fields = self.arguments.len();
                        errors.error(TranspilerError::ArgumentCountMismatch {
                            name: format!("{}()", self.identifier.as_str()),
                            expected: required_fields,
                            found: provided_fields
                        });
                        // Use a placeholder to allow compilation to continue
                        final_args.push(format!("{}: /* ERROR: missing field */", member.ident.transpile(ctx, scope, errors)));
                    }
                }
                
                final_args.join(", ")
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
        let postfix = match self.expression.infer_owned(ctx, scope, errors) {
            Ownership::SharedOwned => ".clone()",
            Ownership::UniqueOwned => "",
            Ownership::Borrowed | Ownership::MutBorrowed | Ownership::Ref => ".to_owned()",
        };
        transpile!(ctx, scope, errors, "{}: {}{postfix}", self.ident, self.expression)
    }
}
