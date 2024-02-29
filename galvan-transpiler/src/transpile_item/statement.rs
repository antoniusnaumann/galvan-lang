use crate::context::Context;
use crate::macros::{impl_transpile_variants, transpile};
use crate::type_inference::InferType;
use crate::{Body, Transpile};
use galvan_ast::{
    BooleanLiteral, DeclModifier, Declaration, Expression, InfixExpression, Literal, NumberLiteral, Ownership, PostfixExpression, Statement, StringLiteral, TypeElement
};
use galvan_resolver::{Scope, Variable};
use itertools::Itertools;

impl Transpile for Body {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let mut body_scope = Scope::child(scope);
        let scope = &mut body_scope;

        let last = match self.statements.last() {
            Some(Statement::Declaration(_)) | Some(Statement::Assignment(_)) => ";",
            _ => "",
        };

        format!(
            "{{\n{}\n}}",
            self.statements
                .iter()
                .map(|stmt| stmt.transpile(ctx, scope))
                .join(";\n")
                + last
        )
    }
}

impl_transpile_variants!(Statement; Assignment, Expression, Declaration /*, Block */);

impl Transpile for Declaration {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let keyword = match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Ref => "let",
            DeclModifier::Mut => "let mut",
        };

        let identifier = self.identifier.transpile(ctx, scope);

        let inferred_type = self.type_annotation.clone().or_else(|| {
            self.assignment
                .as_ref()
                .and_then(|expr| expr.infer_type(scope))
        });

        let ty = inferred_type
            .as_ref()
            .map(|ty| transpile!(ctx, scope, "{}", ty));
        let ty = match self.decl_modifier {
            DeclModifier::Let | DeclModifier::Mut => ty.map_or("".into(), |ty| format!(": {ty}")),
            DeclModifier::Ref => {
                format!(
                    ": std::sync::Arc<std::sync::Mutex<{}>>",
                    ty.unwrap_or("_".into()),
                )
            }
        };

        // TODO: Infer type here
        scope.declare_variable(Variable {
            ident: self.identifier.clone(),
            modifier: self.decl_modifier,
            ty: inferred_type.clone(),
            ownership: match self.decl_modifier {
                DeclModifier::Let | DeclModifier::Mut => match inferred_type {
                    Some(TypeElement::Plain(plain)) if ctx.mapping.is_copy(&plain.ident) => {
                        Ownership::Copy
                    }
                    _ => Ownership::Owned,
                },
                DeclModifier::Ref => Ownership::Ref,
            },
        });

        // TODO: Wrap non-ref types in Arc<Mutex<>> when assigned to a ref type, clone ref types
        // TODO: Clone inner type from ref types to non-ref types
        self.assignment
            .as_ref()
            .map(|expr| transpile_assignment_expression(ctx, expr, scope))
            .map(|expr| {
                if matches!(self.decl_modifier, DeclModifier::Ref) {
                    format!("(&({expr})).__to_ref()")
                } else {
                    expr
                }
            })
            .map(|expr| format!("{keyword} {identifier}{ty} = {expr}"))
            .unwrap_or_else(|| format!("{keyword} {identifier}{ty}"))
    }
}

macro_rules! match_ident {
    ($p:pat) => {
        Expression::Ident($p)
    };
}
pub(crate) use match_ident;

fn transpile_assignment_expression(
    ctx: &Context,
    assigned: &Expression,
    scope: &mut Scope,
) -> String {
    match assigned {
        match_ident!(ident) => {
            transpile!(ctx, scope, "{}.to_owned()", ident)
        }
        Expression::Infix(InfixExpression::Member(access)) if access.is_field() => {
            transpile!(ctx, scope, "{}.to_owned()", access)
        }
        expr => expr.transpile(ctx, scope),
    }
}

impl_transpile_variants! { Expression;
    Ident,
    Infix,
    Postfix,
    CollectionLiteral,
    FunctionCall,
    ConstructorCall,
    ElseExpression,
    Literal,
    Closure,
    Group,
}

impl_transpile_variants! { PostfixExpression;
    YeetExpression,
    AccessExpression,
}
