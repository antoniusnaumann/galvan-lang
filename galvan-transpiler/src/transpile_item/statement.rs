use std::borrow::Borrow;

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

        let inferred_type = self
            .type_annotation
            .clone()
            .or_else(|| self.assignment.as_ref().map(|expr| expr.infer_type(scope)))
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
        scope.declare_variable(Variable {
            ident: self.identifier.clone(),
            modifier: self.decl_modifier,
            ty: inferred_type.clone(),
            ownership,
        });

        let mut scope = Scope::child(scope).returns(inferred_type, ownership);
        // TODO: Wrap non-ref types in Arc<Mutex<>> when assigned to a ref type, clone ref types
        // TODO: Clone inner type from ref types to non-ref types
        self.assignment
            .as_ref()
            .map(|expr| transpile_assignment_expression(ctx, &expr, &mut scope))
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
