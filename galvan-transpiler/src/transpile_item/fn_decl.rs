use crate::context::Context;
use crate::macros::{impl_transpile, transpile};
use crate::transpile_item::ident::TypeOwnership;
use crate::{FnDecl, FnSignature, Param, ParamList, Transpile};
use galvan_ast::{DeclModifier, Ownership, TypeElement};
use galvan_resolver::{Scope, Variable};

impl Transpile for FnDecl {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let mut function_scope = Scope::child(scope);
        function_scope.return_type = self.signature.return_type.clone();
        let scope = &mut function_scope;

        let signature = self.signature.transpile(ctx, scope);
        let block = self.body.transpile(ctx, scope);
        if self.signature.return_type.is_some() {
            transpile!(ctx, scope, "{} {}", signature, block)
        } else {
            transpile!(ctx, scope, "{} {{ {}; }}", signature, block)
        }
    }
}

impl Transpile for FnSignature {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let visibility = self.visibility.transpile(ctx, scope);
        let identifier = self.identifier.transpile(ctx, scope);
        let parameters = self.parameters.transpile(ctx, scope);
        format!(
            "{} fn {}{}{}",
            visibility,
            identifier,
            parameters,
            self.return_type
                .as_ref()
                .map_or("".into(), |return_type| transpile!(
                    ctx,
                    scope,
                    " -> {}",
                    return_type
                ))
        )
    }
}

impl_transpile!(ParamList, "({})", params);

macro_rules! transpile_type {
    ($self:ident, $ctx:ident, $scope:ident, $ownership:path, $prefix:expr) => {{
        use crate::transpile_item::ident::TranspileType;
        let ty = match &$self.param_type {
            TypeElement::Plain(plain) => plain.ident.transpile_type($ctx, $scope, $ownership),
            other => other.transpile($ctx, $scope),
        };

        transpile!($ctx, $scope, "{}: {} {}", &$self.identifier, $prefix, ty)
    }};

    ($self:ident, $ctx:ident, $scope:ident, $ownership:path, $prefix:expr, $prefix_copy:expr) => {{
        use crate::transpile_item::ident::TranspileType;
        let (prefix, ty) = match &$self.param_type {
            TypeElement::Plain(plain) => (
                if $ctx.mapping.is_copy(&plain.ident) {
                    $prefix_copy
                } else {
                    $prefix
                },
                plain.ident.transpile_type($ctx, $scope, $ownership),
            ),
            other => ($prefix, other.transpile($ctx, $scope)),
        };

        transpile!($ctx, $scope, "{}: {} {}", &$self.identifier, prefix, ty)
    }};
}

impl Transpile for Param {
    fn transpile(&self, ctx: &Context, scope: &mut Scope) -> String {
        let is_self = self.identifier.as_str() == "self";

        scope.declare_variable(Variable {
            ident: self.identifier.clone(),
            modifier: self.decl_modifier.unwrap_or(DeclModifier::Let),
            ty: Some(self.param_type.clone()),
            ownership: match self.decl_modifier {
                Some(DeclModifier::Let) | None => match self.param_type {
                    TypeElement::Plain(ref plain) if ctx.mapping.is_copy(&plain.ident) => {
                        Ownership::Copy
                    }
                    _ => Ownership::Borrowed,
                },
                Some(DeclModifier::Mut) => Ownership::MutBorrowed,
                Some(DeclModifier::Ref) => Ownership::Ref,
            },
        });

        match self.decl_modifier {
            Some(DeclModifier::Let) | None => {
                if is_self {
                    "&self".into()
                } else {
                    transpile_type!(self, ctx, scope, TypeOwnership::Borrowed, "&", "")
                }
            }
            Some(DeclModifier::Mut) => {
                if is_self {
                    "&mut self".into()
                } else {
                    transpile_type!(self, ctx, scope, TypeOwnership::MutBorrowed, "&mut")
                }
            }
            Some(DeclModifier::Ref) => {
                if is_self {
                    panic!("Functions with ref-receivers should be handled elsewhere!")
                }

                transpile!(
                    ctx,
                    scope,
                    "{}: std::sync::Arc<std::sync::Mutex<{}>>",
                    self.identifier,
                    self.param_type
                )
            }
        }
    }
}
