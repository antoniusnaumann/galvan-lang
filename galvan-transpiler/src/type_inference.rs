use galvan_ast::{
    ArithmeticOperator, ArrayLiteral, ArrayTypeItem, BasicTypeItem, Block, Body, CollectionLiteral,
    CollectionOperator, DictLiteral, DictLiteralElement, DictionaryTypeItem, ElseExpression,
    Expression, ExpressionKind, FunctionCall, Group, InfixExpression, InfixOperation, Literal,
    MemberOperator, NeverTypeItem, OptionalTypeItem, OrderedDictLiteral, OrderedDictionaryTypeItem,
    Ownership, PostfixExpression, SetLiteral, SetTypeItem, Span, Statement, TypeDecl, TypeElement,
    TypeIdent, UnwrapOperator,
};
use galvan_resolver::{Lookup, Scope};
use itertools::Itertools;

use crate::{
    builtins::{CheckBuiltins, IsSame},
    context::Context,
};

pub(crate) trait InferType {
    fn infer_type(&self, scope: &Scope) -> TypeElement;

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership;
}

impl InferType for ElseExpression {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        let receiver_type = self.receiver.infer_type(scope);
        let block_type = self.block.infer_type(scope);

        match (receiver_type, block_type) {
            (TypeElement::Optional(receiver), TypeElement::Never(_)) => receiver.inner,
            (ty, TypeElement::Never(_)) | (TypeElement::Never(_), ty) => ty,
            (receiver_type, block_type) if receiver_type.is_same(&block_type) => receiver_type,
            (receiver_type, TypeElement::Infer(_)) => receiver_type,
            (TypeElement::Optional(receiver_type), block_type) => {
                if receiver_type.inner.is_same(&block_type) {
                    block_type
                } else {
                    todo!("TRANSPILER ERROR: Types of if and else expression don't match. (allow this when type unions are implemented)")
                }
            }
            (TypeElement::Infer(_), block_type) => block_type,
            (_, _) => todo!("TRANSPILER ERROR: Types of if and else expression don't match."),
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        self.block.infer_owned(ctx, scope)
    }
}

impl InferType for Block {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        // TODO: Block should have access to its inner scope
        self.body.infer_type(scope)
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        self.body.infer_owned(ctx, scope)
    }
}

impl InferType for Body {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        self.statements
            .last()
            .map_or_else(|| TypeElement::void(), |stmt| stmt.infer_type(scope))
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        self.statements
            .last()
            .map(|stmt| stmt.infer_owned(ctx, scope))
            .unwrap_or_default()
    }
}

impl InferType for Statement {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        match self {
            Statement::Assignment(_) => TypeElement::void(),
            Statement::Expression(expr) => expr.infer_type(scope),
            Statement::Declaration(_) => TypeElement::void(),
            Statement::Return(ret) => {
                if ret.is_explicit {
                    TypeElement::Never(NeverTypeItem { span: ret.span })
                } else {
                    ret.expression.infer_type(scope)
                }
            }
            Statement::Throw(throw) => TypeElement::Never(NeverTypeItem { span: throw.span }), // Statement::Block(block) => block.infer_type(scope),
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match self {
            Statement::Expression(expr) => expr.infer_owned(ctx, scope),
            _ => Ownership::SharedOwned,
        }
    }
}

impl InferType for Expression {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        self.kind.infer_type(scope)
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        self.kind.infer_owned(ctx, scope)
    }
}

impl InferType for ExpressionKind {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        match self {
            ExpressionKind::Closure(_) => {
                // todo!("Implement type inference for closure")
                TypeElement::infer()
            }
            ExpressionKind::ElseExpression(e) => e.infer_type(scope),
            ExpressionKind::CollectionLiteral(collection) => collection.infer_type(scope),
            ExpressionKind::FunctionCall(call) => call.infer_type(scope),
            ExpressionKind::ConstructorCall(constructor) => BasicTypeItem {
                ident: TypeIdent::new(constructor.identifier.clone()),
                span: Span::default(),
            }
            .into(),
            ExpressionKind::EnumAccess(access) => TypeElement::Plain(BasicTypeItem {
                ident: access.target.clone(),
                span: Span::default(),
            }),
            ExpressionKind::Literal(literal) => literal.infer_type(scope),
            ExpressionKind::Ident(ident) => {
                let Some(var) = scope.get_variable(ident) else {
                    return TypeElement::infer();
                };
                // println!("cargo::warning=got {:?} named {}", ty, ident);
                var.ty.clone()
            }
            ExpressionKind::Postfix(postfix) => postfix.infer_type(scope),
            ExpressionKind::Infix(operation) => operation.infer_type(scope),
            ExpressionKind::Group(Group { inner }) => inner.infer_type(scope),
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match self {
            ExpressionKind::ElseExpression(e) => {
                let if_owned = e.receiver.infer_owned(ctx, scope);
                let else_owned = e.block.infer_owned(ctx, scope);

                match (if_owned, else_owned) {
                    // TODO: is this correct?
                    (a, b) if a == b => a,
                    _ => Ownership::Borrowed,
                }
            }
            ExpressionKind::FunctionCall(call) => call.infer_owned(ctx, scope),
            ExpressionKind::Infix(infix) => infix.infer_owned(ctx, scope),
            ExpressionKind::Postfix(postfix) => postfix.infer_owned(ctx, scope),
            ExpressionKind::CollectionLiteral(_) => Ownership::UniqueOwned,
            // TODO: this might be copy
            ExpressionKind::ConstructorCall(_) => Ownership::UniqueOwned,
            // TODO: this might be copy
            ExpressionKind::EnumAccess(_) => Ownership::SharedOwned,
            ExpressionKind::Literal(literal) => literal.infer_owned(ctx, scope),
            ExpressionKind::Ident(ident) => {
                let var = scope.get_variable(ident);
                if let Some(var) = var {
                    var.ownership
                } else {
                    todo!("TRANSPILER ERROR: No variable with name {ident} in scope")
                }
            }
            ExpressionKind::Closure(closure) => todo!(),
            ExpressionKind::Group(group) => todo!(),
        }
    }
}

impl InferType for FunctionCall {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        if self.identifier.as_str() == "if" {
            self.arguments
                .last()
                .map(|last| {
                    let ExpressionKind::Closure(ref closure) = last.expression.kind else {
                        panic!("'if' is missing body")
                    };
                    let ty = closure
                        .block
                        .body
                        .statements
                        .last()
                        .map(|stmt| stmt.infer_type(scope))
                        .unwrap_or_default();

                    match ty {
                        TypeElement::Never(_) | TypeElement::Void(_) => ty,
                        ty => TypeElement::Optional(
                            OptionalTypeItem {
                                inner: ty,
                                span: Span::default(),
                            }
                            .into(),
                        ),
                    }
                })
                .unwrap_or_default()
        } else if self.identifier.as_str() == "for" {
            // TODO: infer type of last statement
            Box::new(ArrayTypeItem {
                elements: TypeElement::infer(),
                span: Span::default(),
            })
            .into()
        } else {
            let func = scope.resolve_function(None, &self.identifier, &[]);

            if let Some(func) = func {
                func.signature.return_type.clone()
            } else {
                TypeElement::infer()
            }
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        if self.identifier.as_str() == "if" {
            self.arguments
                .last()
                .map(|last| {
                    let ExpressionKind::Closure(ref closure) = last.expression.kind else {
                        panic!("'if' is missing body")
                    };
                    closure
                        .block
                        .body
                        .statements
                        .last()
                        .map(|stmt| stmt.infer_owned(ctx, scope))
                        .unwrap_or_default()
                })
                .unwrap_or_default()
        } else {
            let func = scope.resolve_function(None, &self.identifier, &[]);

            if let Some(_func) = func {
                // TODO: lookup function, some Rust functions return borrowed values
                Ownership::UniqueOwned
            } else {
                // In galvan, function return values are always owned and since they would be dropped if not consumed, they are considered "unique", that is, never require cloning
                Ownership::UniqueOwned
            }
        }
    }
}

impl InferType for Literal {
    fn infer_type(&self, _scope: &Scope) -> TypeElement {
        match self {
            Literal::BooleanLiteral(_) => TypeElement::bool(),
            Literal::StringLiteral(_) => BasicTypeItem {
                ident: TypeIdent::new("String"),
                span: Span::default(),
            }
            .into(),
            Literal::NumberLiteral(n) => match n.value.parse::<i64>() {
                // TODO: make this more sophisticated, parse into the smallest integer possible
                Ok(_) => BasicTypeItem {
                    ident: TypeIdent::new("Int"),
                    span: Span::default(),
                }
                .into(),
                Err(_) => BasicTypeItem {
                    ident: TypeIdent::new("__Number"),
                    span: Span::default(),
                }
                .into(),
            },
            Literal::NoneLiteral(_) => Box::new(OptionalTypeItem {
                inner: TypeElement::infer(),
                span: Span::default(),
            })
            .into(),
        }
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        match self {
            Literal::StringLiteral(_) => Ownership::UniqueOwned,
            Literal::NumberLiteral(_) | Literal::BooleanLiteral(_) => Ownership::UniqueOwned,
            Literal::NoneLiteral(_) => Ownership::UniqueOwned,
        }
    }
}

impl InferType for InfixExpression {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        match self {
            InfixExpression::Logical(_) => TypeElement::bool(),
            InfixExpression::Arithmetic(e) => e.infer_type(scope),
            InfixExpression::Collection(e) => e.infer_type(scope),
            InfixExpression::Comparison(_) => TypeElement::bool(),
            InfixExpression::Member(e) => e.infer_type(scope),
            InfixExpression::Unwrap(u) => u.infer_type(scope),
            InfixExpression::Custom(_) => todo!("Infer type for custom operators!"),
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match self {
            InfixExpression::Logical(_) => Ownership::UniqueOwned,
            InfixExpression::Arithmetic(_) => {
                // TODO: check the arguments, if they are owned, then this should not be copy
                Ownership::UniqueOwned
            }
            InfixExpression::Collection(collection) => collection.infer_owned(ctx, scope),
            InfixExpression::Comparison(_) => Ownership::UniqueOwned,
            InfixExpression::Member(mem) => {
                // TODO: check if the field is copy to distinguish copy and owned here
                mem.lhs.infer_owned(ctx, scope)
            }
            InfixExpression::Unwrap(u) => u.infer_owned(ctx, scope),
            InfixExpression::Custom(custom) => todo!(),
        }
    }
}

impl InferType for InfixOperation<ArithmeticOperator> {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        let first = self.lhs.infer_type(scope);
        let second = self.rhs.infer_type(scope);

        if let (TypeElement::Plain(a), TypeElement::Plain(b)) = (&first, &second) {
            if a.ident != b.ident && !a.ident.is_intrinsic() && !b.ident.is_intrinsic() {
                todo!("TRANSPILER ERROR: Operands are expected to be of the same type, but were: '{:#?}' and '{:#?}'. \nThis will later be relaxed by automatically lifting the more restrictive operand", a, b)
            }
        }

        let result = if first.is_infer() { second } else { first };

        result
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        todo!()
    }
}

impl InferType for InfixOperation<CollectionOperator> {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        let Self {
            lhs,
            operator,
            rhs: _,
        } = self;

        match operator {
            CollectionOperator::Concat => lhs.infer_type(scope),
            CollectionOperator::Remove => lhs.infer_type(scope),
            CollectionOperator::Contains => TypeElement::bool(),
        }
    }

    fn infer_owned(&self, _ctx: &Context<'_>, _scope: &Scope) -> Ownership {
        match self.operator {
            CollectionOperator::Concat
            | CollectionOperator::Remove
            | CollectionOperator::Contains => Ownership::UniqueOwned,
        }
    }
}

impl InferType for InfixOperation<MemberOperator> {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        let Self {
            lhs,
            operator: _,
            rhs,
        } = self;

        let receiver_type = lhs.infer_type(scope);

        match receiver_type {
            TypeElement::Plain(ty) => {
                let Some(ty) = &scope.resolve_type(&ty.ident) else {
                    return TypeElement::infer();
                };
                let ref ty = ty.item;

                // println!("cargo::warning=resolved: {:?}", ty);
                match self.field_ident() {
                    Some(field) => {
                        // println!("cargo::warning=field: {:?}", field);

                        match ty {
                            TypeDecl::Tuple(tuple) => {
                                todo!("IMPLEMENT: Access member of tuple type")
                            }
                            TypeDecl::Struct(st) => st
                                .members
                                .iter()
                                .find(|member| member.ident == *field)
                                .map(|member| member.r#type.clone())
                                .expect("TRANSPILER ERROR: struct does not have this field"),
                            TypeDecl::Enum(_) => {
                                todo!("TRANSPILER ERROR: Enum cases are access with ::")
                            }
                            TypeDecl::Alias(_) => {
                                // TODO: Handle Inference for alias types
                                TypeElement::infer()
                            }
                            TypeDecl::Empty(_) => {
                                todo!("TRANSPILER ERROR: Cannot access member of empty type")
                            }
                        }
                    }
                    None => {
                        if let ExpressionKind::FunctionCall(ref call) = rhs.kind {
                            println!("cargo::warning=functions: {:?}", scope.functions());
                            if let Some(func) =
                                scope.resolve_function(Some(ty.ident()), &call.identifier, &[])
                            {
                                func.item.signature.return_type.clone()
                            } else {
                                println!("cargo::warning=Function '{}' not found", call.identifier);
                                TypeElement::infer()
                            }
                        } else {
                            panic!("TRANSPILER ERROR: member operator on invalid expression kind")
                        }
                    }
                }
            }
            TypeElement::Optional(_) | TypeElement::Result(_) => {
                // TODO: Handle inference for optional and result types
                // TODO: Ultimately transition to a compiler error here
                //  that tells the user to use safe-call ?. or forward-error-call !.
                TypeElement::infer()
            }
            other => {
                if self.is_field() && !other.is_infer() {
                    todo!(
                        "TRANSPILER ERROR: Cannot access member of type {:#?}",
                        other
                    )
                } else {
                    // TODO: take receiver type here and lookup function
                    TypeElement::infer()
                }
            }
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        todo!()
    }
}

impl InferType for InfixOperation<UnwrapOperator> {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        let ty = self.rhs.infer_type(scope);

        match ty {
            TypeElement::Plain(_) if ty.is_number() => (),
            TypeElement::Infer(_) | TypeElement::Never(_) => (),
            _ => return ty,
        };

        match self.lhs.infer_type(scope) {
            TypeElement::Optional(opt) => opt.inner.clone(),
            TypeElement::Result(res) => res.success.clone(),
            ty @ TypeElement::Infer(_) => ty.clone(),
            _ => todo!("TRANSPILER ERROR: can only use '?' operator on result or optional"),
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match (
            self.rhs.infer_owned(ctx, scope),
            self.lhs.infer_owned(ctx, scope),
        ) {
            (lhs, rhs) if rhs == lhs => rhs,
            (
                Ownership::SharedOwned | Ownership::UniqueOwned,
                Ownership::SharedOwned | Ownership::UniqueOwned,
            ) => Ownership::SharedOwned,
            (Ownership::Borrowed, _) | (_, Ownership::Borrowed) => Ownership::Borrowed,
            _ => todo!("TRANSPILER ERROR: incompatible ownership types in '?' operator"),
        }
    }
}

impl InferType for PostfixExpression {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        match self {
            PostfixExpression::YeetExpression(yeet) => {
                // TODO: Check if return type is matching
                match yeet.inner.infer_type(scope) {
                        TypeElement::Optional(res) => res.inner,
                        TypeElement::Result(res) => res.success,
                        TypeElement::Infer(_) => TypeElement::infer(),
                        _ => todo!("TRANSPILER_ERROR: Yeet operator can only be used on result or optional types"),
                }
            }
            PostfixExpression::AccessExpression(access) => match access.base.infer_type(scope) {
                TypeElement::Array(array) => array.elements,
                TypeElement::Dictionary(dict) => dict.value,
                TypeElement::OrderedDictionary(dict) => dict.value,
                TypeElement::Set(set) => set.elements,
                TypeElement::Infer(_) => TypeElement::infer(),
                _ => {
                    todo!("TRANSPILER_ERROR: Access operator can only be used on collection types")
                }
            },
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        match self {
            PostfixExpression::YeetExpression(yeet_expression) => {
                yeet_expression.inner.infer_owned(ctx, scope)
            }
            PostfixExpression::AccessExpression(access_expression) => {
                access_expression.base.infer_owned(ctx, scope)
            }
        }
    }
}

impl InferType for CollectionLiteral {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        match self {
            CollectionLiteral::ArrayLiteral(array) => array.infer_type(scope),
            CollectionLiteral::DictLiteral(dict) => dict.infer_type(scope),
            CollectionLiteral::SetLiteral(set) => set.infer_type(scope),
            CollectionLiteral::OrderedDictLiteral(ordered_dict) => ordered_dict.infer_type(scope),
        }
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        todo!()
    }
}

impl InferType for ArrayLiteral {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        let elements = infer_from_elements(&self.elements, scope);
        Box::new(ArrayTypeItem {
            elements,
            span: Span::default(),
        })
        .into()
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        todo!()
    }
}

impl InferType for SetLiteral {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        let elements = infer_from_elements(&self.elements, scope);
        Box::new(SetTypeItem {
            elements,
            span: Span::default(),
        })
        .into()
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        todo!()
    }
}

impl InferType for DictLiteral {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        let (key, value) = infer_dict_elements(&self.elements, scope);
        Box::new(DictionaryTypeItem {
            key,
            value,
            span: Span::default(),
        })
        .into()
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        todo!()
    }
}

impl InferType for OrderedDictLiteral {
    fn infer_type(&self, scope: &Scope) -> TypeElement {
        let (key, value) = infer_dict_elements(&self.elements, scope);
        Box::new(OrderedDictionaryTypeItem {
            key,
            value,
            span: Span::default(),
        })
        .into()
    }

    fn infer_owned(&self, ctx: &Context<'_>, scope: &Scope) -> Ownership {
        todo!()
    }
}

fn infer_dict_elements(
    elements: &[DictLiteralElement],
    scope: &Scope,
) -> (TypeElement, TypeElement) {
    let keys = elements.iter().map(|element| &element.key).collect_vec();
    let values = elements.iter().map(|element| &element.value).collect_vec();

    let key = infer_from_elements(keys, scope);
    let value = infer_from_elements(values, scope);

    (key, value)
}

fn infer_from_elements<'a, I>(elements: I, scope: &Scope) -> TypeElement
where
    I: IntoIterator<Item = &'a Expression>,
{
    let inner = elements
        .into_iter()
        .map(|item| item.infer_type(scope))
        .filter(|item| !item.is_infer())
        .unique()
        .collect::<Vec<_>>();

    match inner.len() {
        0 => TypeElement::infer(),
        1 => inner.into_iter().next().unwrap(),
        _ => todo!("TRANSPILE ERROR: Cannot infer type of array literal with multiple types"),
    }
}
