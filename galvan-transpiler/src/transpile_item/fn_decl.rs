use crate::builtins::CheckBuiltins;
use crate::context::Context;
use crate::error::ErrorCollector;
use crate::macros::{impl_transpile, transpile};
use crate::transpile_item::ident::TypeOwnership;
use crate::{FnDecl, FnSignature, Param, ParamList, Transpile};
use galvan_ast::{
    AstNode, CmdDecl, CmdSignature, DeclModifier, Ownership, Return, Statement, TypeElement,
};
use galvan_resolver::{Scope, Variable};

impl Transpile for FnDecl {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let mut function_scope =
            Scope::child(scope).returns(self.signature.return_type.clone(), Ownership::UniqueOwned);
        function_scope.fn_return = self.signature.return_type.clone();
        let scope = &mut function_scope;

        let signature = self.signature.transpile(ctx, scope, errors);

        let mut body = self.body.clone();
        {
            if let Some(stmt) = body.statements.last_mut() {
                if let Statement::Expression(ref expression) = stmt {
                    *stmt = Statement::Return(Return {
                        expression: expression.to_owned(),
                        is_explicit: false,
                        span: expression.span(),
                    })
                }
            }
        };

        let block = body.transpile(ctx, scope, errors);
        if !self.signature.return_type.is_void() {
            transpile!(ctx, scope, errors, "{} {}", signature, block)
        } else {
            transpile!(ctx, scope, errors, "{} {{ {}; }}", signature, block)
        }
    }
}

impl Transpile for FnSignature {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let visibility = self.visibility.transpile(ctx, scope, errors);
        let identifier = self.identifier.transpile(ctx, scope, errors);

        // Collect generic parameters from the function signature
        let generics = self.collect_generics();
        let generic_params = if generics.is_empty() {
            String::new()
        } else {
            let params = generics
                .iter()
                .map(|g| crate::capitalize_generic(g.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            format!("<{}>", params)
        };

        let parameters = self.parameters.transpile(ctx, scope, errors);

        let return_type = match &self.return_type {
            TypeElement::Infer(_) => {
                // TODO: Add proper error handling for function return type inference
                format!("")
            }
            TypeElement::Void(_) => format!(""),
            ty => transpile!(ctx, scope, errors, " -> {}", ty),
        };

        let where_clause = if let Some(where_clause) = &self.where_clause {
            let constraints = where_clause
                .bounds
                .iter()
                .flat_map(|bound| {
                    let trait_bounds = bound
                        .bounds
                        .iter()
                        .map(|b| b.as_str())
                        .collect::<Vec<_>>()
                        .join(" + ");
                    bound.type_params.iter().map(move |p| {
                        format!(
                            "{}: {}",
                            crate::capitalize_generic(p.as_str()),
                            trait_bounds
                        )
                    })
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!(" where {}", constraints)
        } else {
            String::new()
        };

        format!(
            "{visibility} fn {identifier}{generic_params}{parameters}{return_type}{where_clause}",
        )
    }
}

impl Transpile for CmdDecl {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // Generate the command function signature and body
        let mut cmd_scope = Scope::child(scope).returns(
            galvan_ast::TypeElement::void(),
            galvan_ast::Ownership::UniqueOwned,
        );
        let fn_signature = self.signature.transpile(ctx, &mut cmd_scope, errors);
        let fn_body = self.body.transpile(ctx, &mut cmd_scope, errors);

        format!("{} {}", fn_signature, fn_body)
    }
}

impl Transpile for CmdSignature {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // For CLI commands, we transpose to a regular function signature
        // CLI commands are always private and return ()
        let identifier = self.identifier.transpile(ctx, scope, errors);

        // Convert parameters, ignoring short_name for the function signature
        let mut regular_params = Vec::new();
        for param in &self.parameters.params {
            let param_str = format!(
                "{}: {}",
                param.identifier.transpile(ctx, scope, errors),
                param.param_type.transpile(ctx, scope, errors)
            );
            regular_params.push(param_str);
        }
        let parameters = format!("({})", regular_params.join(", "));

        // CLI commands always have void return type
        format!("fn {}{}", identifier, parameters)
    }
}

impl_transpile!(ParamList, "({})", params);

macro_rules! transpile_type {
    ($self:ident, $ctx:ident, $scope:ident, $errors:ident, $ownership:path) => {{
        use crate::transpile_item::ident::TranspileType;
        let mut prefix = "";
        let ty = match &$self.param_type {
            TypeElement::Plain(plain) => plain
                .ident
                .transpile_type($ctx, $scope, $ownership, $errors),
            other => {
                match $ownership {
                    TypeOwnership::Borrowed => prefix = "&",
                    _ => (),
                }
                other.transpile($ctx, $scope, $errors)
            }
        };

        transpile!(
            $ctx,
            $scope,
            $errors,
            "{}: {}{}",
            &$self.identifier,
            prefix,
            ty
        )
    }};
}

impl Transpile for Param {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        let is_self = self.identifier.as_str() == "self";
        let is_copy = ctx.mapping.is_copy(&self.param_type);

        scope.declare_variable(Variable {
            ident: self.identifier.clone(),
            modifier: self.decl_modifier.unwrap_or(DeclModifier::Let),
            ty: self.param_type.clone(),
            ownership: match self.decl_modifier {
                Some(DeclModifier::Let) | None => {
                    if is_copy {
                        Ownership::UniqueOwned
                    } else {
                        Ownership::Borrowed
                    }
                }
                Some(DeclModifier::Mut) => Ownership::MutBorrowed,
                Some(DeclModifier::Ref) => Ownership::Ref,
            },
        });

        match self.decl_modifier {
            Some(DeclModifier::Let) | None => {
                if is_self {
                    if is_copy {
                        "self".into()
                    } else {
                        "&self".into()
                    }
                } else {
                    let ownership = if is_copy {
                        TypeOwnership::Owned
                    } else {
                        TypeOwnership::Borrowed
                    };

                    transpile_type!(self, ctx, scope, errors, ownership)
                }
            }
            Some(DeclModifier::Mut) => {
                if is_self {
                    "&mut self".into()
                } else {
                    transpile_type!(self, ctx, scope, errors, TypeOwnership::MutBorrowed)
                }
            }
            Some(DeclModifier::Ref) => {
                if is_self {
                    panic!("Functions with ref-receivers should be handled elsewhere!")
                }

                transpile!(
                    ctx,
                    scope,
                    errors,
                    "{}: std::sync::Arc<std::sync::Mutex<{}>>",
                    self.identifier,
                    self.param_type
                )
            }
        }
    }
}
