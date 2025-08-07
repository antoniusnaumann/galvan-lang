use std::borrow::Borrow;

use crate::builtins::CheckBuiltins;
use crate::cast::cast;
use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_variants, transpile};
use crate::type_inference::InferType;
use crate::{Body, Transpile};
use galvan_ast::{
    DeclModifier, Declaration, Expression, ExpressionKind, Group, InfixExpression, Ownership,
    PostfixExpression, Return, Statement, Throw, TypeElement,
};
use galvan_resolver::{Scope, Variable};

impl Transpile for Body {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let mut body_scope =
            Scope::child(scope).returns(scope.return_type.to_owned(), scope.ownership);
        let scope = &mut body_scope;

        match self.statements.last() {
            Some(Statement::Expression(expression)) => {
                let len = self.statements.len();
                let last_index = if len > 0 { len - 1 } else { len };
                let statements = transpile!(ctx, scope, "{}", self.statements[0..last_index],);
                let expr = transpile!(ctx, scope, "{}", expression,);
                format!("{{\n{statements};\n{expr}\n}}")
            }
            Some(Statement::Return(expression)) => {
                let len = self.statements.len();
                let last_index = if len > 0 { len - 1 } else { len };
                let statements = transpile!(ctx, scope, "{}", self.statements[0..last_index],);
                let expr = transpile!(ctx, scope, "{}", expression,);
                format!("{{\n{statements};\n{expr}\n}}")
            }
            _ => transpile!(ctx, scope, "{{\n{};\n}}", self.statements,),
        }
    }
}

impl crate::Transpile for Statement {
    fn transpile(&self, ctx: &crate::Context, scope: &mut crate::Scope) -> String {
        match self {
            Statement::Assignment(inner) => inner.transpile(ctx, scope),
            Statement::Expression(inner) => {
                let mut inner_scope =
                    Scope::child(scope).returns(TypeElement::void(), Ownership::UniqueOwned);
                inner.transpile(ctx, &mut inner_scope)
            }
            Statement::Declaration(inner) => inner.transpile(ctx, scope),
            Statement::Return(inner) => inner.transpile(ctx, scope),
            Statement::Throw(inner) => inner.transpile(ctx, scope),
        }
    }
}

impl Transpile for Declaration {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let keyword = match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Ref => "let",
            DeclModifier::Mut => "let mut",
        };

        let identifier = self.identifier.transpile(ctx, scope);

        // IMPORTANT: Evaluate the assignment expression BEFORE adding the variable to scope
        // This ensures that the assignment expression sees the outer scope, not the new variable
        let assignment_result = self.assignment.as_ref().map(|expr| {
            // Create a child scope for evaluating the assignment expression
            let mut assignment_scope = Scope::child(scope);
            
            // If we have a type annotation, set up the scope to expect that type for casting
            if let Some(ref type_annotation) = self.type_annotation {
                assignment_scope.return_type = type_annotation.clone();
                assignment_scope.ownership = match self.decl_modifier {
                    DeclModifier::Let | DeclModifier::Mut => {
                        if ctx.mapping.is_copy(type_annotation) {
                            Ownership::UniqueOwned
                        } else {
                            Ownership::SharedOwned
                        }
                    }
                    DeclModifier::Ref => Ownership::Ref,
                };
            } else {
                // No type annotation - need to infer from expression
                let inferred_type = expr.infer_type(scope);
                
                // Special handling for for expressions that might infer as just "Infer" 
                // instead of "Array(Infer)" due to type inference issues
                let corrected_type = if let ExpressionKind::FunctionCall(func) = &expr.kind {
                    if func.identifier.as_str() == "for" && inferred_type.is_infer() {
                        // Force for expressions to be treated as Array(Infer) when they incorrectly infer as just Infer
                        TypeElement::Array(Box::new(galvan_ast::ArrayTypeItem {
                            elements: TypeElement::infer(),
                            span: galvan_ast::Span::default(),
                        }))
                    } else {
                        inferred_type.clone()
                    }
                } else {
                    inferred_type.clone()
                };
                
                assignment_scope.return_type = corrected_type.clone();
                assignment_scope.ownership = match self.decl_modifier {
                    DeclModifier::Let | DeclModifier::Mut => {
                        if ctx.mapping.is_copy(&corrected_type) {
                            Ownership::UniqueOwned
                        } else {
                            Ownership::SharedOwned
                        }
                    }
                    DeclModifier::Ref => Ownership::Ref,
                };
            }
            
            let assignment_expr = transpile_assignment_expression(ctx, expr, &mut assignment_scope);
            let assignment_type = expr.infer_type(scope); // Use outer scope for type inference
            (assignment_expr, assignment_type)
        });

        // Now determine the inferred type, preferring assignment type over annotation
        let inferred_type = self
            .type_annotation
            .clone()
            .or_else(|| assignment_result.as_ref().map(|(_, ty)| ty.clone()))
            .expect("variables either need a type annotation or an assignment that can be used to infer the type");

        let ty = transpile!(ctx, scope, "{}", inferred_type);
        let ty = match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Mut => format!(": {ty}"),
            DeclModifier::Ref => {
                format!(": std::sync::Arc<std::sync::Mutex<{}>>", ty)
            }
        };

        let ownership = match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Mut => {
                if ctx.mapping.is_copy(&inferred_type) {
                    Ownership::UniqueOwned
                } else {
                    Ownership::SharedOwned
                }
            }
            DeclModifier::Ref => Ownership::Ref,
        };

        // NOW add the variable to scope, after evaluating the assignment
        scope.declare_variable(Variable {
            ident: self.identifier.clone(),
            modifier: self.decl_modifier,
            ty: inferred_type.clone(),
            ownership,
        });

        // Format the final declaration
        assignment_result
            .map(|(expr, _)| {
                let final_expr = if matches!(self.decl_modifier, DeclModifier::Ref) {
                    format!("(&({expr})).__to_ref()")
                } else {
                    expr
                };
                format!("{keyword} {identifier}{ty} = {final_expr}")
            })
            .unwrap_or_else(|| format!("{keyword} {identifier}{ty}"))
    }
}

impl Transpile for Return {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let prefix = if self.is_explicit { "return " } else { "" };

        format!(
            "{prefix}{}",
            cast(
                &self.expression,
                &scope.fn_return.clone(),
                Ownership::UniqueOwned,
                ctx,
                scope,
            )
        )
    }
}

impl Transpile for Throw {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        transpile!(ctx, scope, "return Err({})", self.expression)
    }
}

macro_rules! match_ident {
    ($p:pat) => {
        ExpressionKind::Ident($p)
    };
}
pub(crate) use match_ident;

fn transpile_assignment_expression(
    ctx: &Context,
    assigned: &Expression,
    scope: &mut Scope,
) -> String {
    // TODO: Don't do this, it does not work with implicit ok-wrapping
    match &assigned.kind {
        match_ident!(ident) => return transpile!(ctx, scope, "{}.to_owned()", ident),
        ExpressionKind::Infix(infix) => match infix.borrow() {
            InfixExpression::Member(access) if access.is_field() => {
                return transpile!(ctx, scope, "{}.to_owned()", access)
            }
            _ => (),
        },
        _ => (),
    };

    cast(
        assigned,
        &scope.return_type.clone(),
        scope.ownership,
        ctx,
        scope,
    )
}

impl_transpile_variants! { ExpressionKind;
    Ident,
    Infix,
    Postfix,
    CollectionLiteral,
    FunctionCall,
    ConstructorCall,
    EnumAccess,
    ElseExpression,
    Literal,
    Closure,
    Group,
}

impl_transpile!(Expression, "{}", kind);

impl_transpile!(Group, "({})", inner);

impl_transpile_variants! { PostfixExpression;
    YeetExpression,
    AccessExpression,
}
