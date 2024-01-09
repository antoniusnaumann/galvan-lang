use crate::context::Context;
use crate::macros::{impl_transpile_variants, transpile};
use crate::{Body, Transpile};
use galvan_ast::{
    BasicTypeItem, BooleanLiteral, DeclModifier, Declaration, Expression, NumberLiteral, Ownership,
    Statement, StringLiteral, TypeElement, TypeIdent,
};
use galvan_resolver::{Scope, Variable};
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

        let inferred_type = self.type_annotation.clone().or_else(|| {
            self.expression
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
            ty: inferred_type,
            ownership: match self.decl_modifier {
                DeclModifier::Let(_) | DeclModifier::Mut(_) => match self.type_annotation {
                    Some(TypeElement::Plain(ref plain)) if ctx.mapping.is_copy(&plain.ident) => {
                        Ownership::Copy
                    }
                    _ => Ownership::Owned,
                },
                DeclModifier::Ref(_) => Ownership::Ref,
            },
        });

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

trait InferType {
    fn infer_type(&self, scope: &mut Scope) -> Option<TypeElement>;
}
impl InferType for Expression {
    fn infer_type(&self, scope: &mut Scope) -> Option<TypeElement> {
        match self {
            Expression::ElseExpression(_) => {
                // todo!("Implement type inference for else expression")
                None
            }
            Expression::Closure(_) => {
                // todo!("Implement type inference for closure")
                None
            }
            Expression::CollectionOperation(_) => {
                // todo!("Implement type inference for collection operation")
                None
            }
            Expression::ArithmeticOperation(_) => {
                // todo!("Implement type inference for arithmetic operation")
                None
            }
            Expression::FunctionCall(_) => {
                // todo!("Implement type inference for function call")
                None
            }
            Expression::ConstructorCall(constructor) => Some(constructor.identifier.clone().into()),
            Expression::MemberFunctionCall(_) => {
                // todo!("Implement type inference for member function call")
                None
            }
            Expression::MemberFieldAccess(field) => {
                // todo!("Implement type inference for member field access")
                None
            }
            Expression::BooleanLiteral(_)
            | Expression::LogicalOperation(_)
            | Expression::ComparisonOperation(_) => Some(
                BasicTypeItem {
                    ident: TypeIdent::new("Bool"),
                }
                .into(),
            ),
            Expression::StringLiteral(_) => Some(
                BasicTypeItem {
                    ident: TypeIdent::new("String"),
                }
                .into(),
            ),
            Expression::NumberLiteral(_) => {
                // todo!("Add some way to only give partial type inference")
                None
            }
            Expression::Ident(ident) => scope.get_variable(ident)?.ty.clone()?.into(),
        }
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
