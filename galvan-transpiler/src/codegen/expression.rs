use itertools::Itertools;

use galvan_ast::{
    ArithmeticOperator, BitwiseOperator, ComparisonOperator, Ident, LogicalOperator, RangeOperator,
    TypeElement,
};
use galvan_hir::hir::*;
use galvan_resolver::Lookup;

use crate::context::Context;
use crate::macros::transpile;
use crate::sanitize::{mangle_function_name, sanitize_name, sanitize_path};
use crate::ErrorCollector;
use crate::Transpile;

use super::wrap_ref_storage_value;

impl Transpile for HirExpressionKind {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self {
            HirExpressionKind::If(if_expr) => if_expr.transpile(ctx, errors),
            HirExpressionKind::ElseUnwrap(unwrap) => unwrap.transpile(ctx, errors),
            HirExpressionKind::Try(try_expr) => try_expr.transpile(ctx, errors),
            HirExpressionKind::For(for_expr) => for_expr.transpile(ctx, errors),
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
        transpile!(ctx, errors, "{} => {}", self.pattern, self.body)
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
        let access = format!(
            "{}::{}",
            self.target.transpile(ctx, errors),
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
                transpile!(
                    ctx,
                    errors,
                    "assert_eq!({}, {}, {})",
                    lhs,
                    rhs,
                    args(rest, ctx, errors)
                )
            }
            HirAssert::Ne(lhs, rhs, rest) => {
                transpile!(
                    ctx,
                    errors,
                    "assert_ne!({}, {}, {})",
                    lhs,
                    rhs,
                    args(rest, ctx, errors)
                )
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
        let args = self
            .args
            .iter()
            .map(|argument| argument.transpile(ctx, errors))
            .join(", ");
        if let Some(rust_path) = &self.rust_path {
            return format!("{rust_path}({args})");
        }

        let name = mangle_function_name(self.ident.as_str(), &self.labels);
        if let Some(namespace) = &self.namespace {
            return format!("{}::{}({})", sanitize_path(namespace), name, args);
        }

        format!("{}({})", name, args)
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

        if let Some(rust_path) = &self.rust_path {
            let args = std::iter::once(receiver)
                .chain(
                    self.args
                        .iter()
                        .map(|argument| argument.transpile(ctx, errors)),
                )
                .join(", ");
            return format!("{rust_path}({args})");
        }

        if let Some(namespace) = &self.namespace {
            return format!(
                "{{ use {}::*; {}.{}({}) }}",
                sanitize_path(namespace),
                receiver,
                ident,
                args,
            );
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

impl Transpile for HirFieldAccess {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        transpile!(
            ctx,
            errors,
            "{}.{}",
            self.receiver,
            sanitize_name(self.field.as_str()).into_owned()
        )
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
                    Some(namespace) => format!("{{ use {}::*; {call} }}", sanitize_path(namespace)),
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

impl Transpile for HirConstructorCall {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let ident = self.ident.transpile(ctx, errors);
        let args = self
            .args
            .iter()
            .map(|argument| {
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
        let access = format!(
            "{}::{}",
            self.target.transpile(ctx, errors),
            self.case.as_str()
        );

        if self.args.is_empty() {
            access
        } else if self.args.iter().all(|argument| argument.field.is_none()) {
            let args = self
                .args
                .iter()
                .map(|argument| argument.value.transpile(ctx, errors))
                .join(", ");
            format!("{access}({args})")
        } else {
            let args = self
                .args
                .iter()
                .map(|argument| match &argument.field {
                    Some(field) => format!(
                        "{}: {}",
                        field.as_str(),
                        argument.value.transpile(ctx, errors)
                    ),
                    None => argument.value.transpile(ctx, errors),
                })
                .join(", ");
            format!("{access} {{ {args} }}")
        }
    }
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
            HirCollection::Set(items) => format!(
                "::std::collections::HashSet::from([{}])",
                elements(items, ctx, errors)
            ),
            HirCollection::Dict(items) => format!(
                "::std::collections::HashMap::from([{}])",
                dict_elements(items, ctx, errors)
            ),
            HirCollection::OrderedDict(items) => format!(
                "::std::collections::BTreeMap::from([{}])",
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
            ComparisonOperator::Equal => {
                transpile!(ctx, errors, "({}).eq(&{})", self.lhs, self.rhs)
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
                    "::std::sync::Arc::ptr_eq({}, {})",
                    self.lhs,
                    self.rhs
                )
            }
            ComparisonOperator::NotIdentical => {
                transpile!(
                    ctx,
                    errors,
                    "!::std::sync::Arc::ptr_eq({}, {})",
                    self.lhs,
                    self.rhs
                )
            }
        }
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
                // start ..+ interval => start..(start + interval)
                transpile!(ctx, errors, "{}..({} + {})", self.lhs, self.lhs, self.rhs)
            }
        }
    }
}

impl Transpile for HirBinary<CollectionOperator> {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        match self.operator {
            CollectionOperator::Concat(kind) => transpile_concat(self, kind, ctx, errors),
            CollectionOperator::Remove => {
                errors.warning(
                    "The remove operator '--' is not implemented yet".to_string(),
                    None,
                );
                "/* unsupported remove operator */".to_string()
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
        match &self.base.ty {
            TypeElement::Array(_) => {
                transpile!(ctx, errors, "{}[{}]", self.base, self.index)
            }
            TypeElement::Dictionary(_)
            | TypeElement::OrderedDictionary(_)
            | TypeElement::Set(_) => {
                transpile!(ctx, errors, "{}[&{}]", self.base, self.index)
            }
            _ => {
                errors.error(crate::TranspilerError::InvalidOperationOnType {
                    operation: "index access".into(),
                    allowed_types: "collection types".into(),
                });
                "/* invalid index access */".to_string()
            }
        }
    }
}
