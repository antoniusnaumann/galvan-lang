use itertools::Itertools;

use galvan_ast::{
    ArithmeticOperator, BitwiseOperator, ComparisonOperator, Ident, LogicalOperator, Ownership,
    RangeOperator, TypeElement, TypeIdent, UsePath,
};
use galvan_hir::builtins::CheckBuiltins;
use galvan_hir::hir::*;
use galvan_resolver::Lookup;
use galvan_rustdoc::{RustArgConversion, RustReturnConversion};

use crate::context::Context;
use crate::macros::transpile;
use crate::sanitize::{mangle_function_name, sanitize_name, sanitize_path};
use crate::{ErrorCollector, Transpile, TranspilerError};

use super::wrap_ref_storage_value;

impl Transpile for HirExpressionKind {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self {
            HirExpressionKind::If(if_expr) => if_expr.transpile(ctx, errors),
            HirExpressionKind::ElseUnwrap(unwrap) => unwrap.transpile(ctx, errors),
            HirExpressionKind::Try(try_expr) => try_expr.transpile(ctx, errors),
            HirExpressionKind::For(for_expr) => for_expr.transpile(ctx, errors),
            HirExpressionKind::While(while_expr) => while_expr.transpile(ctx, errors),
            HirExpressionKind::Match(match_expr) => match_expr.transpile(ctx, errors),
            HirExpressionKind::Assert(assert) => assert.transpile(ctx, errors),
            HirExpressionKind::Print(print) => print.transpile(ctx, errors),
            HirExpressionKind::FunctionCall(call) => call.transpile(ctx, errors),
            HirExpressionKind::MethodCall(call) => call.transpile(ctx, errors),
            HirExpressionKind::FieldAccess(access) => access.transpile(ctx, errors),
            HirExpressionKind::SafeAccess(access) => access.transpile(ctx, errors),
            HirExpressionKind::ConstructorCall(constructor) => constructor.transpile(ctx, errors),
            HirExpressionKind::EnumConstructor(constructor) => constructor.transpile(ctx, errors),
            HirExpressionKind::EnumAccess(access) => access.transpile(ctx, errors),
            HirExpressionKind::RustConstant(constant) => constant.rust_path.to_string(),
            HirExpressionKind::Literal(literal) => literal.transpile(ctx, errors),
            HirExpressionKind::Variable(ident) => sanitize_name(ident.as_str()).into_owned(),
            HirExpressionKind::Collection(collection) => collection.transpile(ctx, errors),
            HirExpressionKind::Closure(closure) => closure.transpile(ctx, errors),
            HirExpressionKind::Unary(unary) => unary.transpile(ctx, errors),
            HirExpressionKind::Logical(operation) => operation.transpile(ctx, errors),
            HirExpressionKind::Arithmetic(operation) => operation.transpile(ctx, errors),
            HirExpressionKind::Bitwise(operation) => operation.transpile(ctx, errors),
            HirExpressionKind::Comparison(operation) => operation.transpile(ctx, errors),
            HirExpressionKind::CollectionOp(operation) => operation.transpile(ctx, errors),
            HirExpressionKind::Range(operation) => operation.transpile(ctx, errors),
            HirExpressionKind::Index(index) => index.transpile(ctx, errors),
            HirExpressionKind::Yeet(inner) => {
                transpile!(ctx, errors, "{}?", inner)
            }
            HirExpressionKind::Group(inner) => {
                transpile!(ctx, errors, "({})", inner)
            }
            HirExpressionKind::Error(message) => format!("/* {message} */"),
        }
    }
}

impl Transpile for HirIf {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let condition = self.condition.transpile(ctx, errors);
        let then_block = self.then_block.transpile(ctx, errors);

        if self.wraps_optional {
            format!("if {condition} {then_block} else {{ None }}")
        } else if let Some(else_block) = &self.else_block {
            let else_block = else_block.transpile(ctx, errors);
            format!("if {condition} {then_block} else {else_block}")
        } else {
            format!("if {condition} {then_block}")
        }
    }
}

impl Transpile for HirElseUnwrap {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let value_pattern = if self.by_ref {
            "ref __value"
        } else {
            "__value"
        };
        let receiver = self.receiver.transpile(ctx, errors);
        let value = self.value.transpile(ctx, errors);
        let else_block = self.else_block.transpile(ctx, errors);

        match self.kind {
            HirElseUnwrapKind::Optional => {
                format!("if let Some({value_pattern}) = {receiver} {{ {value} }} else {else_block}")
            }
            HirElseUnwrapKind::Result => {
                let err_binding = fallback_binding(&self.err_binding);
                format!(
                    "match {receiver} {{ Ok({value_pattern}) => {{ {value} }}, Err({err_binding}) => {else_block} }}"
                )
            }
        }
    }
}

impl Transpile for HirTry {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let condition = self.condition.transpile(ctx, errors);
        let body = self.body.transpile(ctx, errors);
        let bindings = self
            .ok_bindings
            .iter()
            .map(|binding| sanitize_name(binding.as_str()))
            .join(", ");

        match &self.else_block {
            Some(else_block) => {
                let else_block = else_block.transpile(ctx, errors);
                match self.kind {
                    TryKind::Optional => {
                        format!(
                            "match {condition} {{ Some(({bindings})) => {body}, None => {else_block} }}"
                        )
                    }
                    TryKind::Result => {
                        let err_binding = fallback_binding(&self.err_binding);
                        format!(
                            "match {condition} {{ Ok({bindings}) => {body}, Err({err_binding}) => {else_block} }}"
                        )
                    }
                }
            }
            // Without an else branch, defer to the runtime support function
            None => format!("r#try({condition}, |{bindings}| {body})"),
        }
    }
}

fn fallback_binding(binding: &Option<Ident>) -> String {
    binding
        .as_ref()
        .map(|binding| sanitize_name(binding.as_str()).into_owned())
        .unwrap_or_else(|| "_".into())
}

impl Transpile for HirFor {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let iterable = self.iterable.transpile(ctx, errors);
        let element = for_pattern(&self.bindings);

        let mut statements: Vec<String> = self
            .body
            .statements
            .iter()
            .map(|statement| statement.transpile(ctx, errors))
            .collect();

        match &self.collect {
            None => {
                let block = statements.join(";\n");
                render_for_loop(
                    self.iterable_kind,
                    iterable,
                    element,
                    format!("{{ {block}; }}"),
                )
            }
            Some(elem_ty) => {
                // Collect the value of each iteration into a result vector
                if let Some(last) = statements.last_mut() {
                    *last = format!("__result.push({last})");
                }
                let block = statements.join(";\n");
                let elem_ty = elem_ty.transpile(ctx, errors);
                let loop_body = format!("{{ {block} }}");
                let loop_expr = render_for_loop(self.iterable_kind, iterable, element, loop_body);
                format!(
                    "{{
                let mut __result: ::std::vec::Vec<{elem_ty}> = ::std::vec::Vec::new(); 
                {loop_expr}
                __result
            }}"
                )
            }
        }
    }
}

impl Transpile for HirWhile {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let condition = self.condition.transpile(ctx, errors);
        let mut statements = self
            .body
            .statements
            .iter()
            .map(|statement| statement.transpile(ctx, errors))
            .collect_vec();

        match &self.collect {
            None => format!("while {condition} {{ {}; }}", statements.join(";\n")),
            Some(elem_ty) => {
                if let Some(last) = statements.last_mut() {
                    *last = format!("__result.push({last})");
                }
                let block = statements.join(";\n");
                let elem_ty = elem_ty.transpile(ctx, errors);
                format!(
                    "{{
                let mut __result: ::std::vec::Vec<{elem_ty}> = ::std::vec::Vec::new();
                while {condition} {{ {block} }}
                __result
            }}"
                )
            }
        }
    }
}

fn render_for_loop(
    kind: HirForIterableKind,
    iterable: String,
    element: String,
    body: String,
) -> String {
    match kind {
        HirForIterableKind::Normal => format!("for {element} in {iterable} {body}"),
        HirForIterableKind::Tuple { len } => {
            let fields = (0..len).map(|i| format!("__iterable.{i}")).join(", ");
            format!("{{ let __iterable = {iterable}; for {element} in [{fields}] {body} }}")
        }
    }
}

fn for_pattern(bindings: &[HirForBinding]) -> String {
    let parts = bindings
        .iter()
        .map(|binding| {
            let prefix = if binding.deref { "&" } else { "" };
            format!("{prefix}{}", sanitize_name(binding.ident.as_str()))
        })
        .join(", ");

    if bindings.len() <= 1 {
        parts
    } else {
        format!("({parts})")
    }
}

impl Transpile for HirMatch {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let scrutinee = self.scrutinee.transpile(ctx, errors);
        let scrutinee = if self.uses_variant_tag {
            format!("({scrutinee}).__variant")
        } else {
            scrutinee
        };
        let arms = self
            .arms
            .iter()
            .map(|arm| arm.transpile(ctx, errors))
            .join(",\n");

        format!("match {scrutinee} {{\n{arms}\n}}")
    }
}

impl Transpile for HirMatchArm {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let pattern = self.pattern.transpile(ctx, errors);
        let body = self.body.transpile(ctx, errors);
        if self.binding_conversions.is_empty() {
            return format!("{pattern} => {body}");
        }

        let conversions = self
            .binding_conversions
            .iter()
            .map(|conversion| {
                let ident = sanitize_name(conversion.ident.as_str()).into_owned();
                let value = transpile_rust_return(ident.clone(), conversion.rust_return_conversion);
                format!("let {ident} = {value};")
            })
            .join("\n");
        format!("{pattern} => {{\n{conversions}\n{body}\n}}")
    }
}

impl Transpile for HirMatchPattern {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self {
            HirMatchPattern::Wildcard => "_".to_string(),
            HirMatchPattern::EnumVariant(pattern) => pattern.transpile(ctx, errors),
        }
    }
}

impl Transpile for HirEnumMatchPattern {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let target = self.target.transpile(ctx, errors);
        let target = if enum_has_common_fields(ctx, &self.target) {
            crate::transpile_item::r#struct::enum_tag_name(&target)
        } else {
            target
        };
        let access = format!(
            "{}::{}",
            target,
            self.case.as_str()
        );

        match &self.arguments {
            HirMatchPatternArguments::None => access,
            HirMatchPatternArguments::Tuple(arguments) => {
                let arguments = arguments
                    .iter()
                    .map(|argument| argument.transpile(ctx, errors))
                    .join(", ");
                format!("{access}({arguments})")
            }
            HirMatchPatternArguments::Named(arguments) => {
                let arguments = arguments
                    .iter()
                    .map(|argument| argument.transpile(ctx, errors))
                    .join(", ");
                format!("{access} {{ {arguments} }}")
            }
        }
    }
}

impl Transpile for HirNamedMatchBinding {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let binding = self.binding.transpile(ctx, errors);
        format!("{}: {binding}", sanitize_name(self.field.as_str()))
    }
}

impl Transpile for HirMatchBindingPattern {
    fn transpile(&self, _ctx: &Context, _errors: &mut ErrorCollector) -> String {
        match self {
            HirMatchBindingPattern::Binding(ident) => sanitize_name(ident.as_str()).into_owned(),
            HirMatchBindingPattern::Wildcard => "_".to_string(),
        }
    }
}

impl Transpile for HirAssert {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let args = |args: &[HirExpression], ctx: &Context, errors: &mut ErrorCollector| {
            args.iter()
                .map(|argument| argument.transpile(ctx, errors))
                .join(", ")
        };

        match self {
            HirAssert::Eq(lhs, rhs, rest) => {
                if lhs.ownership == Ownership::Ref || rhs.ownership == Ownership::Ref {
                    format!(
                        "assert!({}, {})",
                        transpile_value_equality(lhs, rhs, ctx, errors),
                        args(rest, ctx, errors)
                    )
                } else {
                    transpile!(
                        ctx,
                        errors,
                        "assert_eq!({}, {}, {})",
                        lhs,
                        rhs,
                        args(rest, ctx, errors)
                    )
                }
            }
            HirAssert::Ne(lhs, rhs, rest) => {
                if lhs.ownership == Ownership::Ref || rhs.ownership == Ownership::Ref {
                    format!(
                        "assert!(!({}), {})",
                        transpile_value_equality(lhs, rhs, ctx, errors),
                        args(rest, ctx, errors)
                    )
                } else {
                    transpile!(
                        ctx,
                        errors,
                        "assert_ne!({}, {}, {})",
                        lhs,
                        rhs,
                        args(rest, ctx, errors)
                    )
                }
            }
            HirAssert::Truthy(arguments) => {
                format!("assert!({})", args(arguments, ctx, errors))
            }
        }
    }
}

impl Transpile for HirPrint {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let args = self
            .args
            .iter()
            .map(|argument| argument.transpile(ctx, errors))
            .join(", ");

        match self.kind {
            PrintKind::Println => format!("println!(\"{{}}\", {args})"),
            PrintKind::Print => format!("print!(\"{{}}\", {args})"),
            PrintKind::Debug => format!("println!(\"{{:?}}\", {args})"),
            PrintKind::Panic => format!("panic!(\"{{}}\", {args})"),
        }
    }
}

impl Transpile for HirFunctionCall {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let args = transpile_rust_arguments(&self.args, call_arg_conversions(self), ctx, errors);
        let rendered = render_call(
            call_rust_path(self),
            self.namespace.as_ref(),
            &self.ident,
            &self.labels,
            &args,
        );
        apply_call_return(self, rendered)
    }
}

/// Renders `path(args)`, `namespace::name(args)`, or `name(args)` depending on
/// whether the call resolved to an imported Rust path, a namespaced call, or a
/// plain local call.
fn render_call(
    rust_path: Option<&str>,
    namespace: Option<&UsePath>,
    ident: &Ident,
    labels: &[Ident],
    args: &str,
) -> String {
    if let Some(rust_path) = rust_path {
        return format!("{rust_path}({args})");
    }

    let name = mangle_function_name(ident.as_str(), labels);
    match namespace {
        Some(namespace) => format!("{}::{}({})", sanitize_path(namespace), name, args),
        None => format!("{name}({args})"),
    }
}

fn call_rust_path(call: &HirFunctionCall) -> Option<&str> {
    call.rust.as_ref().map(|rust| rust.rust_path.as_ref())
}

fn call_arg_conversions(call: &HirFunctionCall) -> &[RustArgConversion] {
    call.rust
        .as_ref()
        .map(|rust| rust.arg_conversions.as_slice())
        .unwrap_or_default()
}

fn apply_call_return(call: &HirFunctionCall, rendered: String) -> String {
    match &call.rust {
        Some(rust) => transpile_rust_return(rendered, rust.return_conversion),
        None => rendered,
    }
}

impl Transpile for HirMethodCall {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let receiver = self.receiver.transpile(ctx, errors);
        let receiver = if self.receiver.adjustments.is_empty() {
            receiver
        } else {
            format!("({receiver})")
        };
        let args = self
            .args
            .iter()
            .map(|argument| argument.transpile(ctx, errors))
            .join(", ");
        let ident = mangle_function_name(self.ident.as_str(), &self.labels);

        if let Some(rust) = &self.rust {
            if let Some(extension_trait) = &rust.extension_trait {
                let args = transpile_rust_arguments_vec(
                    &self.args,
                    rust.arg_conversions.as_slice(),
                    ctx,
                    errors,
                )
                .join(", ");
                let call = format!("{{ use {extension_trait}; {receiver}.{ident}({args}) }}");
                return transpile_rust_return(call, rust.return_conversion);
            }
            let receiver =
                transpile_rust_argument(&self.receiver, rust.receiver_conversion, ctx, errors);
            let args = std::iter::once(receiver)
                .chain(transpile_rust_arguments_vec(
                    &self.args,
                    rust.arg_conversions.as_slice(),
                    ctx,
                    errors,
                ))
                .join(", ");
            let call = format!("{}({args})", rust.rust_path);
            return transpile_rust_return(call, rust.return_conversion);
        }

        if let Some(namespace) = &self.namespace {
            let import = scoped_extension_import(namespace, &self.receiver.ty)
                .unwrap_or_else(|| format!("{}::*", sanitize_path(namespace)));
            return format!("{{ use {}; {}.{}({}) }}", import, receiver, ident, args,);
        }

        if self.receiver_modifier == Some(galvan_ast::DeclModifier::Ref) {
            let receiver_ty = self.receiver.ty.transpile(ctx, errors);
            let args = std::iter::once(receiver)
                .chain(
                    self.args
                        .iter()
                        .map(|argument| argument.transpile(ctx, errors)),
                )
                .join(", ");
            return format!("{}::{}({})", receiver_ty, ident, args);
        }

        format!("{}.{}({})", receiver, ident, args)
    }
}

fn transpile_rust_arguments(
    args: &[HirExpression],
    conversions: &[RustArgConversion],
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    transpile_rust_arguments_vec(args, conversions, ctx, errors).join(", ")
}

fn transpile_rust_arguments_vec(
    args: &[HirExpression],
    conversions: &[RustArgConversion],
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> Vec<String> {
    args.iter()
        .enumerate()
        .map(|(idx, argument)| {
            transpile_rust_argument(
                argument,
                conversions.get(idx).copied().unwrap_or_default(),
                ctx,
                errors,
            )
        })
        .collect()
}

fn transpile_rust_argument(
    argument: &HirExpression,
    conversion: RustArgConversion,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    let rendered = argument.transpile(ctx, errors);
    apply_rust_arg_conversion_for_expr(rendered, argument, conversion)
}

fn apply_rust_arg_conversion(rendered: String, conversion: RustArgConversion) -> String {
    match conversion {
        RustArgConversion::None => rendered,
        RustArgConversion::SharedBorrow => format!("&{rendered}"),
        RustArgConversion::FixedArrayBorrow => format!(
            "{rendered}.as_slice().try_into().expect(\"Galvan array length must match Rust array length\")"
        ),
        RustArgConversion::FixedArrayMutBorrow => format!(
            "{rendered}.as_mut_slice().try_into().expect(\"Galvan array length must match Rust array length\")"
        ),
        RustArgConversion::BoxNew => format!("::std::boxed::Box::new({rendered})"),
        RustArgConversion::RcNew => format!("::std::rc::Rc::new({rendered})"),
    }
}

fn apply_rust_arg_conversion_for_expr(
    rendered: String,
    argument: &HirExpression,
    conversion: RustArgConversion,
) -> String {
    match conversion {
        RustArgConversion::None => rendered,
        RustArgConversion::SharedBorrow
            if matches!(
                argument.adjusted_ownership(),
                Ownership::Borrowed | Ownership::MutBorrowed
            ) =>
        {
            rendered
        }
        conversion => apply_rust_arg_conversion(rendered, conversion),
    }
}

fn transpile_rust_return(rendered: String, conversion: RustReturnConversion) -> String {
    match conversion {
        RustReturnConversion::None => rendered,
        RustReturnConversion::BoxDeref => format!("*({rendered})"),
    }
}

impl Transpile for HirFieldAccess {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let access = transpile!(
            ctx,
            errors,
            "{}.{}",
            self.receiver,
            sanitize_name(self.field.as_str()).into_owned()
        );
        transpile_rust_return(access, self.rust_return_conversion)
    }
}

impl Transpile for HirSafeAccess {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let receiver = self.receiver.transpile(ctx, errors);
        let access = match &self.access {
            SafeAccessKind::Field(field) => {
                format!("__elem__.{}", sanitize_name(field.as_str()).into_owned())
            }
            SafeAccessKind::Call(namespace, ident, labels, args) => {
                let args = args
                    .iter()
                    .map(|argument| argument.transpile(ctx, errors))
                    .join(", ");
                let call = format!(
                    "__elem__.{}({})",
                    mangle_function_name(ident.as_str(), labels),
                    args
                );
                match namespace {
                    Some(namespace) => {
                        let import = scoped_extension_import(namespace, &self.receiver.ty)
                            .unwrap_or_else(|| format!("{}::*", sanitize_path(namespace)));
                        format!("{{ use {import}; {call} }}")
                    }
                    None => call,
                }
            }
        };

        match self.style {
            SafeAccessStyle::RefClone => {
                format!("{receiver}.as_ref().map(|__elem__| {{ ({access}).clone() }})")
            }
            SafeAccessStyle::Clone => {
                format!("{receiver}.map(|__elem__| {{ ({access}).clone() }})")
            }
            SafeAccessStyle::Move => format!("{receiver}.map(|__elem__| {{ {access} }})"),
        }
    }
}

fn scoped_extension_import(namespace: &UsePath, receiver: &TypeElement) -> Option<String> {
    let receiver = match receiver {
        TypeElement::Plain(receiver) => receiver.ident.as_str(),
        TypeElement::Parametric(receiver) => receiver.base_type.as_str(),
        TypeElement::Optional(receiver) => {
            return scoped_extension_import(namespace, &receiver.inner)
        }
        _ => return None,
    };
    Some(format!(
        "{}::{}_Ext",
        sanitize_path(namespace),
        sanitize_name(receiver)
    ))
}

impl Transpile for HirConstructorCall {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let ident = self.ident.transpile(ctx, errors);
        if self.kind == HirConstructorKind::Tuple {
            let args = self
                .args
                .iter()
                .map(|argument| {
                    let value = argument.value.transpile(ctx, errors);
                    apply_rust_arg_conversion(value, argument.rust_arg_conversion)
                })
                .join(", ");
            return format!("{ident}({args})");
        }

        let args = self
            .args
            .iter()
            .map(|argument| {
                if argument.missing_ref_modifier {
                    errors.error(TranspilerError::InvalidSyntax {
                        message: format!(
                            "ref field '{}' requires the `ref` modifier during construction",
                            argument.field
                        ),
                    });
                }
                let value = argument.value.transpile(ctx, errors);
                let value = if argument.store_as_ref {
                    if let Some(field_ty) = constructor_field_type(self, &argument.field, ctx) {
                        wrap_ref_storage_value(value, &argument.value, field_ty)
                    } else {
                        wrap_ref_storage_value(value, &argument.value, &argument.value.ty)
                    }
                } else {
                    value
                };
                let value = apply_rust_arg_conversion(value, argument.rust_arg_conversion);
                format!("{}: {}", sanitize_name(argument.field.as_str()), value)
            })
            .join(", ");
        format!("{ident} {{ {args} }}")
    }
}

fn constructor_field_type<'a>(
    constructor: &HirConstructorCall,
    field: &Ident,
    ctx: &'a Context<'_>,
) -> Option<&'a TypeElement> {
    let ty = ctx.lookup.resolve_type(&constructor.ident)?;
    let galvan_ast::TypeDecl::Struct(decl) = &ty.item else {
        return None;
    };
    decl.members
        .iter()
        .find(|member| member.ident == *field)
        .map(|member| &member.r#type)
}

impl Transpile for HirEnumConstructor {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let target = self.target.transpile(ctx, errors);
        let variant_target = if self.common_args.is_empty() {
            target.clone()
        } else {
            crate::transpile_item::r#struct::enum_tag_name(&target)
        };
        let access = format!(
            "{}::{}",
            variant_target,
            self.case.as_str()
        );

        let variant = if self.args.is_empty() {
            access
        } else if self.args.iter().all(|argument| argument.field.is_none()) {
            let args = self
                .args
                .iter()
                .map(|argument| {
                    let value = argument.value.transpile(ctx, errors);
                    apply_rust_arg_conversion(value, argument.rust_arg_conversion)
                })
                .join(", ");
            format!("{access}({args})")
        } else {
            let args = self
                .args
                .iter()
                .map(|argument| match &argument.field {
                    Some(field) => {
                        let value = argument.value.transpile(ctx, errors);
                        let value = apply_rust_arg_conversion(value, argument.rust_arg_conversion);
                        format!("{}: {}", field.as_str(), value)
                    }
                    None => {
                        let value = argument.value.transpile(ctx, errors);
                        apply_rust_arg_conversion(value, argument.rust_arg_conversion)
                    }
                })
                .join(", ");
            format!("{access} {{ {args} }}")
        };

        if self.common_args.is_empty() {
            return variant;
        }

        let common_args = self
            .common_args
            .iter()
            .map(|argument| {
                if argument.missing_ref_modifier {
                    errors.error(TranspilerError::InvalidSyntax {
                        message: format!(
                            "ref field '{}' requires the `ref` modifier during construction",
                            argument.field
                        ),
                    });
                }
                let value = argument.value.transpile(ctx, errors);
                let value = if argument.store_as_ref {
                    let field_ty = enum_common_field_type(self, &argument.field, ctx)
                        .unwrap_or(&argument.value.ty);
                    wrap_ref_storage_value(value, &argument.value, field_ty)
                } else {
                    value
                };
                format!("{}: {value}", sanitize_name(argument.field.as_str()))
            })
            .chain(std::iter::once(format!("__variant: {variant}")))
            .join(", ");
        format!("{target} {{ {common_args} }}")
    }
}

fn enum_common_field_type<'a>(
    constructor: &HirEnumConstructor,
    field: &Ident,
    ctx: &'a Context<'_>,
) -> Option<&'a TypeElement> {
    let ty = ctx.lookup.resolve_type(&constructor.target)?;
    let galvan_ast::TypeDecl::Enum(decl) = &ty.item else {
        return None;
    };
    decl.common_fields
        .iter()
        .find(|member| member.ident == *field)
        .map(|member| &member.r#type)
}

fn enum_has_common_fields(ctx: &Context<'_>, target: &TypeIdent) -> bool {
    ctx.lookup
        .resolve_type(target)
        .and_then(|ty| match &ty.item {
            galvan_ast::TypeDecl::Enum(decl) => Some(!decl.common_fields.is_empty()),
            _ => None,
        })
        .unwrap_or(false)
}

impl Transpile for HirEnumAccess {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        format!(
            "{}::{}",
            self.target.transpile(ctx, errors),
            self.case.as_str()
        )
    }
}

impl Transpile for HirLiteral {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self {
            HirLiteral::Unit => "()".to_string(),
            HirLiteral::Boolean(value) => format!("{value}"),
            HirLiteral::Number(value) => value.clone(),
            HirLiteral::Char(value) => format!("'{}'", value.escape_default()),
            HirLiteral::None => "None".to_string(),
            HirLiteral::String(string) => string.transpile(ctx, errors),
        }
    }
}

impl Transpile for HirStringLiteral {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        if self.interpolations.is_empty() {
            format!("format!({})", self.value)
        } else {
            let args = self
                .interpolations
                .iter()
                .map(|interpolation| interpolation.transpile(ctx, errors))
                .join(", ");
            format!("format!({}, {})", self.value, args)
        }
    }
}

impl Transpile for HirCollection {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let elements = |elements: &[HirExpression], ctx: &Context, errors: &mut ErrorCollector| {
            elements
                .iter()
                .map(|element| element.transpile(ctx, errors))
                .join(", ")
        };

        match self {
            HirCollection::Array(items) => format!("vec![{}]", elements(items, ctx, errors)),
            HirCollection::Tuple(items) if items.len() == 1 => {
                format!("({},)", elements(items, ctx, errors))
            }
            HirCollection::Tuple(items) => format!("({})", elements(items, ctx, errors)),
            HirCollection::Set(items) => format!(
                "::std::collections::HashSet::from([{}])",
                elements(items, ctx, errors)
            ),
            HirCollection::Dict(items) => format!(
                "::std::collections::HashMap::from([{}])",
                dict_elements(items, ctx, errors)
            ),
            HirCollection::OrderedDict(items) => format!(
                "::galvan::std::IndexMap::from([{}])",
                dict_elements(items, ctx, errors)
            ),
        }
    }
}

fn dict_elements(
    elements: &[HirDictElement],
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    elements
        .iter()
        .map(|element| {
            format!(
                "({}, {})",
                element.key.transpile(ctx, errors),
                element.value.transpile(ctx, errors)
            )
        })
        .join(", ")
}

impl Transpile for HirClosure {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| {
                let prefix = if parameter.deref { "&" } else { "" };
                format!("{prefix}{}", sanitize_name(parameter.ident.as_str()))
            })
            .join(", ");
        let body = self.body.transpile(ctx, errors);
        format!("|{parameters}| {body}")
    }
}

impl Transpile for HirUnary {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self.operator {
            galvan_ast::UnaryOperator::LogicalNot => {
                transpile!(ctx, errors, "!({})", self.operand)
            }
        }
    }
}

impl Transpile for HirBinary<LogicalOperator> {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self.operator {
            LogicalOperator::And => transpile!(ctx, errors, "{} && {}", self.lhs, self.rhs),
            LogicalOperator::Or => transpile!(ctx, errors, "{} || {}", self.lhs, self.rhs),
            LogicalOperator::Xor => transpile!(ctx, errors, "{} ^ {}", self.lhs, self.rhs),
        }
    }
}

impl Transpile for HirBinary<ArithmeticOperator> {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self.operator {
            ArithmeticOperator::Add => transpile!(ctx, errors, "{} + {}", self.lhs, self.rhs),
            ArithmeticOperator::Sub => transpile!(ctx, errors, "{} - {}", self.lhs, self.rhs),
            ArithmeticOperator::Mul => transpile!(ctx, errors, "{} * {}", self.lhs, self.rhs),
            ArithmeticOperator::Div => transpile!(ctx, errors, "{} / {}", self.lhs, self.rhs),
            ArithmeticOperator::Rem => transpile!(ctx, errors, "{} % {}", self.lhs, self.rhs),
            ArithmeticOperator::Exp
                if self.lhs.ty.is_number()
                    && matches!(
                        &self.result_ty,
                        TypeElement::Plain(result)
                            if result.ident.as_str() != "__Number"
                    ) =>
            {
                transpile!(
                    ctx,
                    errors,
                    "({} as {}).pow({})",
                    self.lhs,
                    self.result_ty,
                    self.rhs
                )
            }
            ArithmeticOperator::Exp if self.lhs.ty.is_number() && self.result_ty.is_number() => {
                transpile!(ctx, errors, "({} as i64).pow({})", self.lhs, self.rhs)
            }
            ArithmeticOperator::Exp => transpile!(ctx, errors, "{}.pow({})", self.lhs, self.rhs),
        }
    }
}

impl Transpile for HirBinary<BitwiseOperator> {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self.operator {
            BitwiseOperator::Or => transpile!(ctx, errors, "{} | {}", self.lhs, self.rhs),
            BitwiseOperator::And => transpile!(ctx, errors, "{} & {}", self.lhs, self.rhs),
            BitwiseOperator::Xor => transpile!(ctx, errors, "{} ^ {}", self.lhs, self.rhs),
            BitwiseOperator::ShiftLeft => transpile!(ctx, errors, "{} << {}", self.lhs, self.rhs),
            BitwiseOperator::ShiftRight => transpile!(ctx, errors, "{} >> {}", self.lhs, self.rhs),
        }
    }
}

impl Transpile for HirBinary<ComparisonOperator> {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self.operator {
            ComparisonOperator::Equal
                if self.lhs.ownership == Ownership::Ref || self.rhs.ownership == Ownership::Ref =>
            {
                transpile_value_equality(&self.lhs, &self.rhs, ctx, errors)
            }
            ComparisonOperator::Equal => {
                transpile!(ctx, errors, "({}).eq(&{})", self.lhs, self.rhs)
            }
            ComparisonOperator::NotEqual
                if self.lhs.ownership == Ownership::Ref || self.rhs.ownership == Ownership::Ref =>
            {
                format!(
                    "!({})",
                    transpile_value_equality(&self.lhs, &self.rhs, ctx, errors)
                )
            }
            ComparisonOperator::NotEqual => {
                transpile!(ctx, errors, "({}).ne(&{})", self.lhs, self.rhs)
            }
            ComparisonOperator::Less => transpile!(ctx, errors, "({}).lt(&{})", self.lhs, self.rhs),
            ComparisonOperator::LessEqual => {
                transpile!(ctx, errors, "({}).le(&{})", self.lhs, self.rhs)
            }
            ComparisonOperator::Greater => {
                transpile!(ctx, errors, "({}).gt(&{})", self.lhs, self.rhs)
            }
            ComparisonOperator::GreaterEqual => {
                transpile!(ctx, errors, "({}).ge(&{})", self.lhs, self.rhs)
            }
            ComparisonOperator::Identical => {
                transpile!(
                    ctx,
                    errors,
                    "::std::sync::Arc::ptr_eq(&{}, &{})",
                    self.lhs,
                    self.rhs
                )
            }
            ComparisonOperator::NotIdentical => {
                transpile!(
                    ctx,
                    errors,
                    "!::std::sync::Arc::ptr_eq(&{}, &{})",
                    self.lhs,
                    self.rhs
                )
            }
        }
    }
}

fn transpile_value_equality(
    lhs: &HirExpression,
    rhs: &HirExpression,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    let lhs_is_ref = lhs.ownership == Ownership::Ref;
    let rhs_is_ref = rhs.ownership == Ownership::Ref;
    let lhs = lhs.transpile(ctx, errors);
    let rhs = rhs.transpile(ctx, errors);
    match (lhs_is_ref, rhs_is_ref) {
        (true, true) => {
            format!("::galvan::std::__ref_value_eq(&({lhs}), &({rhs}))")
        }
        (true, false) => {
            format!("{{ let __left = &({lhs}); (*__left.lock().unwrap()).eq(&({rhs})) }}")
        }
        (false, true) => {
            format!("{{ let __right = &({rhs}); ({lhs}).eq(&*__right.lock().unwrap()) }}")
        }
        (false, false) => format!("({lhs}).eq(&({rhs}))"),
    }
}

impl Transpile for HirBinary<RangeOperator> {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self.operator {
            RangeOperator::Inclusive => {
                transpile!(ctx, errors, "{}..=({})", self.lhs, self.rhs)
            }
            RangeOperator::Exclusive => {
                transpile!(ctx, errors, "{}..({})", self.lhs, self.rhs)
            }
            RangeOperator::Tolerance => {
                // center ± tolerance => (center - tolerance)..=(center + tolerance)
                transpile!(
                    ctx,
                    errors,
                    "({} - {})..=({} + {})",
                    self.lhs,
                    self.rhs,
                    self.lhs,
                    self.rhs
                )
            }
            RangeOperator::Interval => {
                // start ..+ interval => start..=(start + interval)
                transpile!(ctx, errors, "{}..=({} + {})", self.lhs, self.lhs, self.rhs)
            }
            RangeOperator::Descending => {
                transpile!(
                    ctx,
                    errors,
                    "(({} - {})..=({})).rev()",
                    self.lhs,
                    self.rhs,
                    self.lhs
                )
            }
        }
    }
}

impl Transpile for HirBinary<CollectionOperator> {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self.operator {
            CollectionOperator::Concat(kind) => transpile_concat(self, kind, ctx, errors),
            CollectionOperator::Remove(RemoveKind::Array) => transpile!(
                ctx,
                errors,
                "{{ let mut result = ({}).to_owned(); for removed in ({}).iter() {{ if let Some(index) = result.iter().position(|item| item == removed) {{ result.remove(index); }} }} result }}",
                self.lhs,
                self.rhs
            ),
            CollectionOperator::Repeat(RepeatKind::Array | RepeatKind::String) => {
                transpile!(
                    ctx,
                    errors,
                    "({}).repeat(({}) as usize)",
                    self.lhs,
                    self.rhs
                )
            }
            CollectionOperator::Repeat(RepeatKind::Char) => {
                transpile!(
                    ctx,
                    errors,
                    "({}).to_string().repeat(({}) as usize)",
                    self.lhs,
                    self.rhs
                )
            }
            CollectionOperator::Contains => {
                transpile!(ctx, errors, "({}).contains(&({}))", self.rhs, self.lhs)
            }
        }
    }
}

/// `++` concatenation; the shape was decided by the typechecker, the
/// collection kind comes from the stored left-hand side type
fn transpile_concat(
    operation: &HirBinary<CollectionOperator>,
    kind: ConcatKind,
    ctx: &Context,
    errors: &mut ErrorCollector,
) -> String {
    match (&operation.lhs.ty, kind) {
        (TypeElement::Array(_), ConcatKind::Element) => {
            transpile!(
                ctx,
                errors,
                "{{ let mut temp = ({}).to_owned(); temp.push({}); temp }}",
                operation.lhs,
                operation.rhs
            )
        }
        (TypeElement::Set(_), ConcatKind::Element) => {
            transpile!(
                ctx,
                errors,
                "{{ let mut temp = ({}).to_owned(); temp.insert({}); temp }}",
                operation.lhs,
                operation.rhs
            )
        }
        (TypeElement::Set(_), _) => {
            transpile!(
                ctx,
                errors,
                "({}).union(&{}).cloned().collect::<::std::collections::HashSet<_>>()",
                operation.lhs,
                operation.rhs
            )
        }
        (TypeElement::Plain(basic), ConcatKind::Element) if basic.ident.as_str() == "String" => {
            transpile!(
                ctx,
                errors,
                "{{ let mut temp = ({}).to_owned(); temp.push({}); temp }}",
                operation.lhs,
                operation.rhs
            )
        }
        (TypeElement::Plain(basic), _) if basic.ident.as_str() == "String" => {
            transpile!(
                ctx,
                errors,
                "format!(\"{{}}{{}}\" , {}, {})",
                operation.lhs,
                operation.rhs
            )
        }
        // Arrays and unknown collection types concatenate as arrays
        _ => {
            transpile!(
                ctx,
                errors,
                "[({}).to_owned(), ({}).to_owned()].concat()",
                operation.lhs,
                operation.rhs
            )
        }
    }
}

impl Transpile for HirIndex {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match (&self.base.ty, self.kind) {
            (TypeElement::Array(_), IndexKind::Element) => {
                transpile!(ctx, errors, "{}[{}]", self.base, self.index)
            }
            (TypeElement::Array(_), IndexKind::Slice) => {
                transpile!(
                    ctx,
                    errors,
                    "{{ let __base = &({}); ({}).map(|__index| __base[__index as usize].to_owned()).collect::<::std::vec::Vec<_>>() }}",
                    self.base,
                    self.index
                )
            }
            (TypeElement::Plain(plain), IndexKind::Slice) if plain.ident.as_str() == "String" => {
                transpile!(
                    ctx,
                    errors,
                    "{{ let __chars = ({}).chars().collect::<::std::vec::Vec<_>>(); ({}).map(|__index| __chars[__index as usize]).collect::<::std::string::String>() }}",
                    self.base,
                    self.index
                )
            }
            (
                TypeElement::Dictionary(_)
                | TypeElement::OrderedDictionary(_)
                | TypeElement::Set(_),
                IndexKind::Element,
            ) => {
                transpile!(ctx, errors, "{}[&{}]", self.base, self.index)
            }
            _ => {
                errors.error_with_span(
                    crate::TranspilerError::InvalidOperationOnType {
                        operation: "index access".into(),
                        allowed_types: "collection types".into(),
                    },
                    Some(self.base.span.into()),
                );
                "/* invalid index access */".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use galvan_ast::{Ident, Ownership, Span, TypeElement, TypeIdent};
    use galvan_hir::mapping::Mapping;

    use super::*;

    #[test]
    fn tuple_struct_constructors_transpile_as_tuple_calls() {
        let constructor = HirConstructorCall {
            ident: TypeIdent::new("UserId"),
            kind: HirConstructorKind::Tuple,
            args: vec![HirConstructorArg {
                field: Ident::new("value"),
                value: HirExpression::new(
                    HirExpressionKind::Literal(HirLiteral::Number("42".to_string())),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ),
                store_as_ref: false,
                missing_ref_modifier: false,
                rust_arg_conversion: RustArgConversion::None,
            }],
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(constructor.transpile(&ctx, &mut errors), "UserId(42)");
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn struct_constructors_reject_missing_ref_field_modifiers() {
        let constructor = HirConstructorCall {
            ident: TypeIdent::new("Owner"),
            kind: HirConstructorKind::Struct,
            args: vec![HirConstructorArg {
                field: Ident::new("dog"),
                value: HirExpression::new(
                    HirExpressionKind::Variable(Ident::new("dog")),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ),
                store_as_ref: true,
                missing_ref_modifier: true,
                rust_arg_conversion: RustArgConversion::None,
            }],
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        constructor.transpile(&ctx, &mut errors);

        assert!(errors.has_errors());
        assert!(errors
            .to_string()
            .contains("ref field 'dog' requires the `ref` modifier during construction"));
    }

    #[test]
    fn resolved_extension_methods_import_the_specific_trait() {
        let method = HirMethodCall {
            receiver: HirExpression::new(
                HirExpressionKind::Variable(Ident::new("book")),
                TypeElement::Plain(galvan_ast::BasicTypeItem {
                    ident: TypeIdent::new("String"),
                    span: Span::default(),
                }),
                Ownership::UniqueOwned,
                Span::default(),
            ),
            receiver_modifier: None,
            namespace: Some(UsePath {
                segments: vec![Ident::new("reader")],
                span: Span::default(),
            }),
            rust: Some(HirRustMethodCall {
                rust_path: "<::std::string::String as ::reader::String_Ext>::read_and_judge".into(),
                extension_trait: Some("::reader::String_Ext".into()),
                return_conversion: RustReturnConversion::None,
                receiver_conversion: RustArgConversion::SharedBorrow,
                arg_conversions: Vec::new(),
            }),
            ident: Ident::new("read_and_judge"),
            labels: Vec::new(),
            args: Vec::new(),
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            method.transpile(&ctx, &mut errors),
            "{ use ::reader::String_Ext; book.read_and_judge() }"
        );
    }

    #[test]
    fn tuple_struct_constructors_apply_rust_argument_conversions() {
        let constructor = HirConstructorCall {
            ident: TypeIdent::new("TicketPair"),
            kind: HirConstructorKind::Tuple,
            args: vec![
                HirConstructorArg {
                    field: Ident::new("first"),
                    value: HirExpression::new(
                        HirExpressionKind::Variable(Ident::new("first")),
                        TypeElement::infer(),
                        Ownership::UniqueOwned,
                        Span::default(),
                    ),
                    store_as_ref: false,
                    missing_ref_modifier: false,
                    rust_arg_conversion: RustArgConversion::BoxNew,
                },
                HirConstructorArg {
                    field: Ident::new("second"),
                    value: HirExpression::new(
                        HirExpressionKind::Variable(Ident::new("second")),
                        TypeElement::infer(),
                        Ownership::UniqueOwned,
                        Span::default(),
                    ),
                    store_as_ref: false,
                    missing_ref_modifier: false,
                    rust_arg_conversion: RustArgConversion::RcNew,
                },
            ],
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            constructor.transpile(&ctx, &mut errors),
            "TicketPair(::std::boxed::Box::new(first), ::std::rc::Rc::new(second))"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn struct_constructors_apply_rust_argument_conversions() {
        let constructor = HirConstructorCall {
            ident: TypeIdent::new("TicketEnvelope"),
            kind: HirConstructorKind::Struct,
            args: vec![HirConstructorArg {
                field: Ident::new("ticket"),
                value: HirExpression::new(
                    HirExpressionKind::Variable(Ident::new("ticket")),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ),
                store_as_ref: false,
                missing_ref_modifier: false,
                rust_arg_conversion: RustArgConversion::BoxNew,
            }],
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            constructor.transpile(&ctx, &mut errors),
            "TicketEnvelope { ticket: ::std::boxed::Box::new(ticket) }"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn tuple_enum_constructors_apply_rust_argument_conversions() {
        let constructor = HirEnumConstructor {
            target: TypeIdent::new("TicketEvent"),
            case: TypeIdent::new("Assigned"),
            common_args: Vec::new(),
            args: vec![HirEnumConstructorArg {
                field: None,
                value: HirExpression::new(
                    HirExpressionKind::Variable(Ident::new("user")),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ),
                rust_arg_conversion: RustArgConversion::RcNew,
            }],
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            constructor.transpile(&ctx, &mut errors),
            "TicketEvent::Assigned(::std::rc::Rc::new(user))"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn named_enum_constructors_apply_rust_argument_conversions() {
        let constructor = HirEnumConstructor {
            target: TypeIdent::new("TicketEvent"),
            case: TypeIdent::new("Moved"),
            common_args: Vec::new(),
            args: vec![HirEnumConstructorArg {
                field: Some(Ident::new("owner")),
                value: HirExpression::new(
                    HirExpressionKind::Variable(Ident::new("owner")),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ),
                rust_arg_conversion: RustArgConversion::BoxNew,
            }],
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            constructor.transpile(&ctx, &mut errors),
            "TicketEvent::Moved { owner: ::std::boxed::Box::new(owner) }"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn match_arms_apply_rust_binding_conversions() {
        let arm = HirMatchArm {
            pattern: HirMatchPattern::EnumVariant(HirEnumMatchPattern {
                target: TypeIdent::new("TicketEvent"),
                case: TypeIdent::new("Assigned"),
                arguments: HirMatchPatternArguments::Tuple(vec![HirMatchBindingPattern::Binding(
                    Ident::new("user"),
                )]),
            }),
            binding_conversions: vec![HirMatchBindingConversion {
                ident: Ident::new("user"),
                rust_return_conversion: RustReturnConversion::BoxDeref,
            }],
            body: HirBlock {
                statements: vec![HirStatement::Expression(HirExpression::new(
                    HirExpressionKind::Variable(Ident::new("user")),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ))],
                ty: TypeElement::infer(),
                span: Span::default(),
            },
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            arm.transpile(&ctx, &mut errors),
            "TicketEvent::Assigned(user) => {\nlet user = *(user);\n{\nuser\n}\n}"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn rust_calls_apply_shared_borrow_argument_conversions() {
        let call = HirFunctionCall {
            namespace: None,
            rust: Some(HirRustCall {
                rust_path: "::demo::takes_ref".into(),
                return_conversion: RustReturnConversion::None,
                arg_conversions: vec![RustArgConversion::SharedBorrow],
            }),
            ident: Ident::new("takes_ref"),
            labels: Vec::new(),
            args: vec![HirExpression::new(
                HirExpressionKind::Literal(HirLiteral::Number("42".to_string())),
                TypeElement::Plain(galvan_ast::BasicTypeItem {
                    ident: TypeIdent::new("U64"),
                    span: Span::default(),
                }),
                Ownership::UniqueOwned,
                Span::default(),
            )],
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(call.transpile(&ctx, &mut errors), "::demo::takes_ref(&42)");
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn rust_calls_convert_fixed_array_borrows_at_the_boundary() {
        let call = HirFunctionCall {
            namespace: None,
            rust: Some(HirRustCall {
                rust_path: "::demo::reads_arrays".into(),
                return_conversion: RustReturnConversion::None,
                arg_conversions: vec![
                    RustArgConversion::FixedArrayBorrow,
                    RustArgConversion::FixedArrayMutBorrow,
                ],
            }),
            ident: Ident::new("reads_arrays"),
            labels: Vec::new(),
            args: vec![
                HirExpression::new(
                    HirExpressionKind::Variable(Ident::new("values")),
                    TypeElement::infer(),
                    Ownership::Borrowed,
                    Span::default(),
                ),
                HirExpression::new(
                    HirExpressionKind::Variable(Ident::new("mutable_values")),
                    TypeElement::infer(),
                    Ownership::MutBorrowed,
                    Span::default(),
                ),
            ],
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            call.transpile(&ctx, &mut errors),
            "::demo::reads_arrays(values.as_slice().try_into().expect(\"Galvan array length must match Rust array length\"), mutable_values.as_mut_slice().try_into().expect(\"Galvan array length must match Rust array length\"))"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn imported_generic_types_render_with_qualified_path_and_type_args() {
        use galvan_ast::{
            BasicTypeItem, GenericTypeItem, ParametricTypeItem, StructTypeDecl, ToplevelItem,
            TupleTypeDecl, TupleTypeMember, TypeDecl, Visibility,
        };
        use galvan_files::Source;
        use galvan_hir::mapping::RustType;

        // An external tuple struct `Json<T>` imported from axum: registered in the
        // lookup and mapped to its fully-qualified Rust path. The `HealthResponse`
        // type argument is a local struct, so it renders unqualified.
        let json_decl = ToplevelItem {
            item: TypeDecl::Tuple(TupleTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new("Json"),
                generic_params: vec![Ident::new("T")],
                members: vec![TupleTypeMember {
                    r#type: TypeElement::Generic(GenericTypeItem {
                        ident: Ident::new("T"),
                        span: Span::default(),
                    }),
                    span: Span::default(),
                }],
                span: Span::default(),
            }),
            source: Source::Builtin,
        };
        let health_decl = ToplevelItem {
            item: TypeDecl::Struct(StructTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new("HealthResponse"),
                generic_params: Vec::new(),
                members: Vec::new(),
                span: Span::default(),
            }),
            source: Source::Builtin,
        };

        let mut mapping = Mapping::default();
        mapping.types.insert(
            TypeIdent::new("Json"),
            RustType::new("::axum::Json", "::axum::Json", "::axum::Json", false),
        );
        let mut ctx = Context::new(mapping);
        ctx.lookup.types.insert(TypeIdent::new("Json"), &json_decl);
        ctx.lookup
            .types
            .insert(TypeIdent::new("HealthResponse"), &health_decl);
        let mut errors = ErrorCollector::new();

        let plain = TypeElement::Parametric(ParametricTypeItem {
            base_type: TypeIdent::new("Json"),
            type_args: vec![TypeElement::Plain(BasicTypeItem {
                ident: TypeIdent::new("HealthResponse"),
                span: Span::default(),
            })],
            span: Span::default(),
        });
        assert_eq!(
            plain.transpile(&ctx, &mut errors),
            "::axum::Json<HealthResponse>"
        );

        // A Galvan list type argument lowers to `Vec`, still under the qualified path.
        let nested = TypeElement::Parametric(ParametricTypeItem {
            base_type: TypeIdent::new("Json"),
            type_args: vec![TypeElement::Array(Box::new(galvan_ast::ArrayTypeItem {
                elements: TypeElement::Plain(BasicTypeItem {
                    ident: TypeIdent::new("HealthResponse"),
                    span: Span::default(),
                }),
                span: Span::default(),
            }))],
            span: Span::default(),
        });
        assert_eq!(
            nested.transpile(&ctx, &mut errors),
            "::axum::Json<::std::vec::Vec<HealthResponse>>"
        );

        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn rust_associated_function_paths_render_as_rust_paths() {
        let call = HirFunctionCall {
            namespace: None,
            rust: Some(HirRustCall {
                rust_path: "::external::Router::new".into(),
                return_conversion: RustReturnConversion::None,
                arg_conversions: Vec::new(),
            }),
            ident: Ident::new("new"),
            labels: Vec::new(),
            args: Vec::new(),
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            call.transpile(&ctx, &mut errors),
            "::external::Router::new()"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn rust_associated_function_tuple_arguments_render_as_tuple_arguments() {
        let octets = HirExpression::new(
            HirExpressionKind::Collection(HirCollection::Array(vec![
                HirExpression::new(
                    HirExpressionKind::Literal(HirLiteral::Number("127".to_string())),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ),
                HirExpression::new(
                    HirExpressionKind::Literal(HirLiteral::Number("0".to_string())),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ),
                HirExpression::new(
                    HirExpressionKind::Literal(HirLiteral::Number("0".to_string())),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ),
                HirExpression::new(
                    HirExpressionKind::Literal(HirLiteral::Number("1".to_string())),
                    TypeElement::infer(),
                    Ownership::UniqueOwned,
                    Span::default(),
                ),
            ])),
            TypeElement::infer(),
            Ownership::UniqueOwned,
            Span::default(),
        );
        let port = HirExpression::new(
            HirExpressionKind::Literal(HirLiteral::Number("3000".to_string())),
            TypeElement::infer(),
            Ownership::UniqueOwned,
            Span::default(),
        );
        let call = HirFunctionCall {
            namespace: None,
            rust: Some(HirRustCall {
                rust_path: "::std::net::SocketAddr::from".into(),
                return_conversion: RustReturnConversion::None,
                arg_conversions: Vec::new(),
            }),
            ident: Ident::new("from"),
            labels: Vec::new(),
            args: vec![HirExpression::new(
                HirExpressionKind::Collection(HirCollection::Tuple(vec![octets, port])),
                TypeElement::infer(),
                Ownership::UniqueOwned,
                Span::default(),
            )],
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            call.transpile(&ctx, &mut errors),
            "::std::net::SocketAddr::from((vec![127, 0, 0, 1], 3000))"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn rust_associated_constants_render_as_rust_paths() {
        let constant = HirExpressionKind::RustConstant(HirRustConstant {
            rust_path: "::external::StatusCode::CREATED".into(),
        });
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            constant.transpile(&ctx, &mut errors),
            "::external::StatusCode::CREATED"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn rust_calls_apply_owned_wrapper_argument_conversions() {
        let args = vec![
            HirExpression::new(
                HirExpressionKind::Literal(HirLiteral::Number("42".to_string())),
                TypeElement::Plain(galvan_ast::BasicTypeItem {
                    ident: TypeIdent::new("U64"),
                    span: Span::default(),
                }),
                Ownership::UniqueOwned,
                Span::default(),
            ),
            HirExpression::new(
                HirExpressionKind::Variable(Ident::new("ticket")),
                TypeElement::Plain(galvan_ast::BasicTypeItem {
                    ident: TypeIdent::new("Ticket"),
                    span: Span::default(),
                }),
                Ownership::UniqueOwned,
                Span::default(),
            ),
        ];
        let call = HirFunctionCall {
            namespace: None,
            rust: Some(HirRustCall {
                rust_path: "::demo::takes_wrappers".into(),
                return_conversion: RustReturnConversion::None,
                arg_conversions: vec![RustArgConversion::BoxNew, RustArgConversion::RcNew],
            }),
            ident: Ident::new("takes_wrappers"),
            labels: Vec::new(),
            args,
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            call.transpile(&ctx, &mut errors),
            "::demo::takes_wrappers(::std::boxed::Box::new(42), ::std::rc::Rc::new(ticket))"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn rust_calls_apply_box_return_conversions() {
        let call = HirFunctionCall {
            namespace: None,
            rust: Some(HirRustCall {
                rust_path: "::demo::boxed_ticket".into(),
                return_conversion: RustReturnConversion::BoxDeref,
                arg_conversions: Vec::new(),
            }),
            ident: Ident::new("boxed_ticket"),
            labels: Vec::new(),
            args: Vec::new(),
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(
            call.transpile(&ctx, &mut errors),
            "*(::demo::boxed_ticket())"
        );
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn rust_calls_without_return_conversions_remain_unchanged() {
        let call = HirFunctionCall {
            namespace: None,
            rust: Some(HirRustCall {
                rust_path: "::demo::shared_ticket".into(),
                return_conversion: RustReturnConversion::None,
                arg_conversions: Vec::new(),
            }),
            ident: Ident::new("shared_ticket"),
            labels: Vec::new(),
            args: Vec::new(),
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(call.transpile(&ctx, &mut errors), "::demo::shared_ticket()");
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }

    #[test]
    fn rust_field_access_applies_box_return_conversions() {
        let access = HirFieldAccess {
            receiver: HirExpression::new(
                HirExpressionKind::Variable(Ident::new("envelope")),
                TypeElement::infer(),
                Ownership::UniqueOwned,
                Span::default(),
            ),
            rust_return_conversion: RustReturnConversion::BoxDeref,
            field: Ident::new("ticket"),
        };
        let ctx = Context::new(Mapping::default());
        let mut errors = ErrorCollector::new();

        assert_eq!(access.transpile(&ctx, &mut errors), "*(envelope.ticket)");
        assert!(!errors.has_errors(), "expected no errors, got: {errors}");
    }
}
