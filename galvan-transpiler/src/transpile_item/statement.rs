use crate::context::Context;
use crate::macros::{impl_transpile, impl_transpile_variants, transpile};
use crate::{Body, Transpile};
use galvan_ast::{
    BooleanLiteral, DeclModifier, Declaration, Expression, NumberLiteral, Statement, StringLiteral,
};
use galvan_resolver::Scope;
use itertools::Itertools;

impl Transpile for Body {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let mut body_scope = Scope::child(scope);
        let scope = &mut body_scope;

        format!(
            "{{\n{}\n}}",
            self.statements
                .iter()
                .map(|stmt| stmt.transpile(ctx, scope))
                .join(";\n")
        )
    }
}

impl_transpile_variants!(Statement; Assignment, Expression, Declaration, Block);

impl Transpile for Declaration {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let keyword = match self.decl_modifier {
            DeclModifier::Let(_) | DeclModifier::Ref(_) => "let",
            DeclModifier::Mut(_) => "let mut",
        };

        let identifier = self.identifier.transpile(ctx, scope);
        let ty = self
            .type_annotation
            .as_ref()
            .map(|ty| transpile!(ctx, scope, "{}", ty));
        let ty = match self.decl_modifier {
            DeclModifier::Let(_) | DeclModifier::Mut(_) => {
                ty.map_or("".into(), |ty| format!(": {ty}"))
            }
            DeclModifier::Ref(_) => {
                format!(
                    ": std::sync::Arc<std::sync::Mutex<{}>>",
                    ty.unwrap_or("_".into()),
                )
            }
        };

        // TODO: Wrap non-ref types in Arc<Mutex<>> when assigned to a ref type, clone ref types
        // TODO: Clone inner type from ref types to non-ref types
        self.expression
            .as_ref()
            .map(|expr| transpile_assignment_expression(ctx, expr, scope))
            .map(|expr| {
                if matches!(self.decl_modifier, DeclModifier::Ref(_)) {
                    format!("(&({expr})).__to_ref()")
                } else {
                    expr
                }
            })
            .map(|expr| format!("{keyword} {identifier}{ty} = {expr}"))
            .unwrap_or_else(|| format!("{keyword} {identifier}{ty}"))
    }
}

fn transpile_assignment_expression(ctx: &Context, expr: &Expression, scope: &mut Scope) -> String {
    match expr {
        Expression::Ident(ident) => {
            transpile!(ctx, scope, "{}.to_owned()", ident)
        }
        Expression::MemberFieldAccess(access) => {
            transpile!(ctx, scope, "{}.to_owned()", access)
        }
        expr => expr.transpile(ctx, scope),
    }
}

impl_transpile_variants! { Expression;
    ElseExpression,
    Closure,
    LogicalOperation,
    ComparisonOperation,
    CollectionOperation,
    ArithmeticOperation,
    FunctionCall,
    ConstructorCall,
    MemberFunctionCall,
    MemberFieldAccess,
    BooleanLiteral,
    StringLiteral,
    NumberLiteral,
    Ident
}

impl Transpile for StringLiteral {
    fn transpile(&self, _: &Context, scope: &mut Scope) -> String {
        // TODO: Implement more sophisticated formatting (extract {} and put them as separate arguments)
        format!("format!({})", self.as_str())
    }
}

impl Transpile for NumberLiteral {
    fn transpile(&self, _: &Context, scope: &mut Scope) -> String {
        // TODO: Parse number and validate type
        format!("{}", self.as_str())
    }
}

impl Transpile for BooleanLiteral {
    fn transpile(&self, _: &Context, scope: &mut Scope) -> String {
        format!("{self}")
    }
}
