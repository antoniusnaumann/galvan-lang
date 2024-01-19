use crate::context::Context;
use crate::macros::{impl_transpile_variants, transpile};
use crate::type_inference::InferType;
use crate::{Body, Transpile};
use galvan_ast::{
    BooleanLiteral, DeclModifier, Declaration, Expression, Literal, NumberLiteral, Ownership,
    SingleExpression, Statement, StringLiteral, TopExpression, TypeElement,
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

impl_transpile_variants!(Statement; Assignment, TopExpression, Declaration, Block);

impl Transpile for Declaration {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let keyword = match self.decl_modifier {
            DeclModifier::Let(_) | DeclModifier::Ref(_) => "let",
            DeclModifier::Mut(_) => "let mut",
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

        // TODO: Infer type here
        scope.declare_variable(Variable {
            ident: self.identifier.clone(),
            modifier: self.decl_modifier,
            ty: inferred_type.clone(),
            ownership: match self.decl_modifier {
                DeclModifier::Let(_) | DeclModifier::Mut(_) => match inferred_type {
                    Some(TypeElement::Plain(plain)) if ctx.mapping.is_copy(&plain.ident) => {
                        Ownership::Copy
                    }
                    _ => Ownership::Owned,
                },
                DeclModifier::Ref(_) => Ownership::Ref,
            },
        });

        // TODO: Wrap non-ref types in Arc<Mutex<>> when assigned to a ref type, clone ref types
        // TODO: Clone inner type from ref types to non-ref types
        self.assignment
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

macro_rules! match_ident {
    ($p:pat) => {
        Expression::SingleExpression(SingleExpression::Ident($p))
    };
}
pub(crate) use match_ident;

fn transpile_assignment_expression(
    ctx: &Context,
    assigned: &TopExpression,
    scope: &mut Scope,
) -> String {
    match assigned {
        TopExpression::Expression(expr) => match expr {
            match_ident!(ident) => {
                transpile!(ctx, scope, "{}.to_owned()", ident)
            }
            Expression::MemberFieldAccess(access) => {
                transpile!(ctx, scope, "{}.to_owned()", access)
            }
            expr => expr.transpile(ctx, scope),
        },
        TopExpression::ElseExpression(e) => e.transpile(ctx, scope),
    }
}

impl_transpile_variants! { Expression;
    OperatorTree,
    MemberFunctionCall,
    MemberFieldAccess,
    SingleExpression,
    Closure
}

impl_transpile_variants! { SingleExpression;
    CollectionLiteral,
    FunctionCall,
    ConstructorCall,
    Literal,
    Ident
}

impl_transpile_variants! { Literal;
    BooleanLiteral,
    StringLiteral,
    NumberLiteral
}

impl Transpile for StringLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope) -> String {
        // TODO: Implement more sophisticated formatting (extract {} and put them as separate arguments)
        format!("format!({})", self.as_str())
    }
}

impl Transpile for NumberLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope) -> String {
        // TODO: Parse number and validate type
        format!("{}", self.as_str())
    }
}

impl Transpile for BooleanLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope) -> String {
        format!("{self}")
    }
}
