use std::borrow::Borrow;

use crate::cast::cast;
use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_variants, transpile};
use crate::type_inference::InferType;
use crate::{Body, Transpile};
use galvan_ast::{
    AstNode, DeclModifier, Declaration, Expression, ExpressionKind, Group, InfixExpression,
    Ownership, PostfixExpression, Return, Statement, Throw, TypeElement,
};
use galvan_resolver::{Scope, Variable};
use itertools::Itertools;

impl Transpile for Body {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let mut body_scope = Scope::child(scope).returns(scope.return_type.to_owned());
        let scope = &mut body_scope;

        let last = match self.statements.last() {
            Some(Statement::Declaration(_)) | Some(Statement::Assignment(_)) => ";",
            _ => "",
        };

        let len = self.statements.len();

        format!(
            "{{\n{}\n}}",
            self.statements
                .iter()
                .enumerate()
                .map(|(i, stmt)| {
                    if i == len - 1 {
                        if let Statement::Expression(expression) = stmt {
                            return Return {
                                expression: expression.to_owned(),
                                is_explicit: false,
                                span: expression.span(),
                            }
                            .transpile(ctx, scope);
                        }
                    };
                    stmt.transpile(ctx, scope)
                })
                .join(";\n")
                + last
        )
    }
}

impl_transpile_variants!(Statement; Assignment, Expression, Declaration, Return, Throw /*, Block */);

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
                DeclModifier::Let | DeclModifier::Mut => {
                    if inferred_type
                        .as_ref()
                        .is_some_and(|ty| ctx.mapping.is_copy(ty))
                    {
                        Ownership::Copy
                    } else {
                        Ownership::Owned
                    }
                }
                DeclModifier::Ref => Ownership::Ref,
            },
        });

        let mut scope = Scope::child(scope)
            .returns(Some(inferred_type.unwrap_or_else(|| TypeElement::infer())));
        // TODO: Wrap non-ref types in Arc<Mutex<>> when assigned to a ref type, clone ref types
        // TODO: Clone inner type from ref types to non-ref types
        self.assignment
            .as_ref()
            .map(|expr| transpile_assignment_expression(ctx, &expr.kind, &mut scope))
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

impl Transpile for Return {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let prefix = if self.is_explicit { "return " } else { "" };

        format!(
            "{prefix}{}",
            cast(&self.expression, &scope.return_type.clone(), ctx, scope)
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
    assigned: &ExpressionKind,
    scope: &mut Scope,
) -> String {
    match assigned {
        match_ident!(ident) => {
            transpile!(ctx, scope, "{}.to_owned()", ident)
        }
        ExpressionKind::Infix(infix) => match infix.borrow() {
            InfixExpression::Member(access) if access.is_field() => {
                transpile!(ctx, scope, "{}.to_owned()", access)
            }
            expr => expr.transpile(ctx, scope),
        },
        expr => expr.transpile(ctx, scope),
    }
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
