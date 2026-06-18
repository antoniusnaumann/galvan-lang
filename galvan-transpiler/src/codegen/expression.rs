use galvan_ast::{
    ArithmeticOperator, ComparisonOperator, LogicalOperator, RangeOperator, TypeElement,
};
use galvan_hir::hir::*;
use itertools::Itertools;

use crate::context::Context;
use crate::macros::transpile;
use crate::sanitize::sanitize_name;
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
            HirExpressionKind::Assert(assert) => assert.transpile(ctx, errors),
            HirExpressionKind::Print(print) => print.transpile(ctx, errors),
            HirExpressionKind::FunctionCall(call) => call.transpile(ctx, errors),
            HirExpressionKind::MethodCall(call) => call.transpile(ctx, errors),
            HirExpressionKind::FieldAccess(access) => access.transpile(ctx, errors),
            HirExpressionKind::SafeAccess(access) => access.transpile(ctx, errors),
            HirExpressionKind::ConstructorCall(constructor) => constructor.transpile(ctx, errors),
            HirExpressionKind::EnumConstructor(constructor) => constructor.transpile(ctx, errors),
            HirExpressionKind::EnumAccess(access) => access.transpile(ctx, errors),
            HirExpressionKind::Literal(literal) => literal.transpile(ctx, errors),
            HirExpressionKind::Variable(ident) => sanitize_name(ident.as_str()).into_owned(),
            HirExpressionKind::Collection(collection) => collection.transpile(ctx, errors),
            HirExpressionKind::Closure(closure) => closure.transpile(ctx, errors),
            HirExpressionKind::Logical(operation) => operation.transpile(ctx, errors),
            HirExpressionKind::Arithmetic(operation) => operation.transpile(ctx, errors),
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
        let pattern = if self.by_ref {
            "ref __value"
        } else {
            "__value"
        };
        transpile!(
            ctx,
            errors,
            "if let Some({pattern}) = {} {{ {} }} else {}",
            self.receiver,
            self.value,
            self.else_block,
        )
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
                        let err_binding = self
                            .err_binding
                            .as_ref()
                            .map(|binding| sanitize_name(binding.as_str()).into_owned())
                            .unwrap_or_else(|| "_".into());
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

impl Transpile for HirFor {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let iterable = self.iterable.transpile(ctx, errors);

        let element = if self.bindings.is_empty() {
            "it".to_string()
        } else {
            format!(
                "({})",
                self.bindings
                    .iter()
                    .map(|binding| sanitize_name(binding.as_str()))
                    .join(", ")
            )
        };
        let prefix = if self.bind_by_ref { "&" } else { "" };

        let mut statements: Vec<String> = self
            .body
            .statements
            .iter()
            .map(|statement| statement.transpile(ctx, errors))
            .collect();

        match &self.collect {
            None => {
                let block = statements.join(";\n");
                format!("for {prefix}{element} in {iterable} {{ {block}; }}")
            }
            Some(elem_ty) => {
                // Collect the value of each iteration into a result vector
                if let Some(last) = statements.last_mut() {
                    *last = format!("__result.push({last})");
                }
                let block = statements.join(";\n");
                let elem_ty = elem_ty.transpile(ctx, errors);
                format!(
                    "{{
                let mut __result: ::std::vec::Vec<{elem_ty}> = ::std::vec::Vec::new(); 
                for {prefix}{element} in {iterable} {{ {block} }}
                __result
            }}"
                )
            }
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
        format!("{}({})", sanitize_name(self.ident.as_str()), args)
    }
}

impl Transpile for HirMethodCall {
    fn transpile(&self, ctx: &Context, errors: &mut ErrorCollector) -> String {
        let receiver = self.receiver.transpile(ctx, errors);
        let args = self
            .args
            .iter()
            .map(|argument| argument.transpile(ctx, errors))
            .join(", ");
        format!(
            "{}.{}({})",
            receiver,
            sanitize_name(self.ident.as_str()),
            args
        )
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
            SafeAccessKind::Field(field) => sanitize_name(field.as_str()).into_owned(),
            SafeAccessKind::Call(ident, args) => {
                let args = args
                    .iter()
                    .map(|argument| argument.transpile(ctx, errors))
                    .join(", ");
                format!("{}({})", sanitize_name(ident.as_str()), args)
            }
        };

        match self.style {
            SafeAccessStyle::RefClone => {
                format!("{receiver}.as_ref().map(|__elem__| {{ __elem__.{access}.clone() }})")
            }
            SafeAccessStyle::Clone => {
                format!("{receiver}.map(|__elem__| {{ __elem__.{access}.clone() }})")
            }
            SafeAccessStyle::Move => format!("{receiver}.map(|__elem__| {{ __elem__.{access} }})"),
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
                    wrap_ref_storage_value(value, &argument.value)
                } else {
                    value
                };
                format!("{}: {}", sanitize_name(argument.field.as_str()), value)
            })
            .join(", ");
        format!("{ident} {{ {args} }}")
    }
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
