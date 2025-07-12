use galvan_ast::{
    ArithmeticOperator, ArrayLiteral, ArrayTypeItem, BasicTypeItem, Block, Body, CollectionLiteral,
    CollectionOperator, DictLiteral, DictLiteralElement, DictionaryTypeItem, ElseExpression,
    Expression, Group, InfixExpression, InfixOperation, Literal, MemberOperator, NeverTypeItem,
    OptionalTypeItem, OrderedDictLiteral, OrderedDictionaryTypeItem, PostfixExpression, SetLiteral,
    SetTypeItem, Span, Statement, TypeDecl, TypeElement, TypeIdent,
};
use galvan_resolver::{Lookup, Scope};
use itertools::Itertools;

use crate::builtins::IsSame;

pub(crate) trait InferType {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement>;
}

impl InferType for ElseExpression {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let receiver_type = self.receiver.infer_type(scope);
        let block_type = self.block.infer_type(scope);

        match (receiver_type, block_type) {
            (Some(receiver_type), Some(block_type)) if receiver_type.is_same(&block_type) => {
                Some(receiver_type)
            }
            (ty, Some(TypeElement::Never(_))) | (Some(TypeElement::Never(_)), ty) => ty,
            (Some(receiver_type), None) => Some(receiver_type),
            (Some(TypeElement::Optional(receiver_type)), Some(block_type)) => {
                if receiver_type.some.is_same(&block_type) {
                    Some(block_type)
                } else {
                    todo!("TRANSPILER ERROR: Types of if and else expression don't match. (allow this when type unions are implemented)")
                }
            }
            (None, Some(block_type)) => Some(block_type),
            (None, None) => None,
            (_, _) => todo!("TRANSPILER ERROR: Types of if and else expression don't match."),
        }
    }
}

impl InferType for Block {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        // TODO: Block should have access to its inner scope
        self.body.infer_type(scope)
    }
}

impl InferType for Body {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        self.statements
            .last()
            .and_then(|stmt| stmt.infer_type(scope))
    }
}

impl InferType for Statement {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        match self {
            Statement::Assignment(_) => None,
            Statement::Expression(expr) => expr.infer_type(scope),
            Statement::Declaration(_) => None,
            Statement::Return(ret) => {
                if ret.is_explicit {
                    Some(TypeElement::Never(NeverTypeItem { span: ret.span }))
                } else {
                    ret.expression.infer_type(scope)
                }
            }
            Statement::Throw(throw) => Some(TypeElement::Never(NeverTypeItem { span: throw.span })), // Statement::Block(block) => block.infer_type(scope),
        }
    }
}

impl InferType for Expression {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        match self {
            Expression::Closure(_) => {
                // todo!("Implement type inference for closure")
                None
            }
            Expression::ElseExpression(e) => e.infer_type(scope),
            Expression::CollectionLiteral(collection) => collection.infer_type(scope),
            Expression::FunctionCall(call) => {
                if call.identifier.as_str() == "if" {
                    call.arguments.last().and_then(|last| {
                        let Expression::Closure(ref closure) = last.expression else {
                            panic!("'if' is missing body")
                        };
                        closure
                            .block
                            .body
                            .statements
                            .last()
                            .and_then(|stmt| stmt.infer_type(scope))
                            .map(|ty| {
                                TypeElement::Optional(
                                    OptionalTypeItem {
                                        some: ty,
                                        span: Span::default(),
                                    }
                                    .into(),
                                )
                            })
                    })
                } else {
                    let func = scope.resolve_function(None, &call.identifier, &[]);

                    if let Some(func) = func {
                        func.signature.return_type.clone()
                    } else {
                        None
                    }
                }
            }
            Expression::ConstructorCall(constructor) => Some(
                BasicTypeItem {
                    ident: TypeIdent::new(constructor.identifier.clone()),
                    span: Span::default(),
                }
                .into(),
            ),
            Expression::Literal(literal) => literal.infer_type(scope),
            Expression::Ident(ident) => {
                let ty = scope.get_variable(ident)?.ty.clone();
                // println!("cargo::warning=got {:?} named {}", ty, ident);
                ty?.into()
            }
            Expression::Postfix(postfix) => postfix.infer_type(scope),
            Expression::Infix(operation) => operation.infer_type(scope),
            Expression::Group(Group { inner, span: _span }) => inner.infer_type(scope),
        }
    }
}

impl InferType for Literal {
    fn infer_type(&self, _scope: &Scope) -> Option<TypeElement> {
        match self {
            Literal::BooleanLiteral(_) => Some(bool()),
            Literal::StringLiteral(_) => Some(
                BasicTypeItem {
                    ident: TypeIdent::new("String"),
                    span: Span::default(),
                }
                .into(),
            ),
            Literal::NumberLiteral(_) => Some(
                BasicTypeItem {
                    ident: TypeIdent::new("__Number"),
                    span: Span::default(),
                }
                .into(),
            ),
            Literal::NoneLiteral(_) => Some(
                Box::new(OptionalTypeItem {
                    some: infer(),
                    span: Span::default(),
                })
                .into(),
            ),
        }
    }
}

fn bool() -> TypeElement {
    BasicTypeItem {
        ident: TypeIdent::new("Bool"),
        span: Span::default(),
    }
    .into()
}

fn infer() -> TypeElement {
    BasicTypeItem {
        ident: TypeIdent::new("__Infer"),
        span: Span::default(),
    }
    .into()
}

impl InferType for InfixExpression {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        match self {
            InfixExpression::Logical(_) => Some(bool()),
            InfixExpression::Arithmetic(e) => e.infer_type(scope),
            InfixExpression::Collection(e) => e.infer_type(scope),
            InfixExpression::Comparison(_) => Some(bool()),
            InfixExpression::Member(e) => e.infer_type(scope),
            InfixExpression::Custom(_) => todo!("Infer type for custom operators!"),
        }
    }
}

impl InferType for InfixOperation<ArithmeticOperator> {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let first = self.lhs.infer_type(scope);
        let second = self.rhs.infer_type(scope);

        if let (Some(TypeElement::Plain(a)), Some(TypeElement::Plain(b))) = (&first, &second) {
            if a.ident != b.ident && !a.ident.is_intrinsic() && !b.ident.is_intrinsic() {
                todo!("TRANSPILER ERROR: Operands are expected to be of the same type, but were: '{:#?}' and '{:#?}'. \nThis will later be relaxed by automatically lifting the more restrictive operand", a, b)
            }
        }

        let result = first.or(second);

        result
    }
}

impl InferType for InfixOperation<CollectionOperator> {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let Self {
            lhs,
            operator,
            rhs: _,
            span: _span,
        } = self;

        match operator {
            CollectionOperator::Concat => lhs.infer_type(scope),
            CollectionOperator::Remove => lhs.infer_type(scope),
            CollectionOperator::Contains => Some(bool()),
        }
    }
}

impl InferType for InfixOperation<MemberOperator> {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let Self {
            lhs,
            operator: _,
            rhs,
            span: _span,
        } = self;

        let receiver_type = lhs.infer_type(scope)?;

        match receiver_type {
            TypeElement::Plain(ty) => {
                let ty = &scope.resolve_type(&ty.ident)?.item;

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
                                .map(|member| member.r#type.clone()),
                            TypeDecl::Alias(_) => {
                                // TODO: Handle Inference for alias types
                                None
                            }
                            TypeDecl::Empty(_) => {
                                todo!("TRANSPILER ERROR: Cannot access member of empty type")
                            }
                        }
                    }
                    None => {
                        if let Expression::FunctionCall(call) = rhs {
                            println!("cargo::warning=functions: {:?}", scope.functions());
                            if let Some(func) =
                                scope.resolve_function(Some(ty.ident()), &call.identifier, &[])
                            {
                                func.item.signature.return_type.clone()
                            } else {
                                println!("cargo::warning=Function '{}' not found", call.identifier);
                                None
                            }
                        } else {
                            None
                        }
                    }
                }
            }
            TypeElement::Optional(_) | TypeElement::Result(_) => {
                // TODO: Handle inference for optional and result types
                // TODO: Ultimately transition to a compiler error here
                //  that tells the user to use safe-call ?. or forward-error-call !.
                None
            }
            other => {
                if self.is_field() {
                    todo!(
                        "TRANSPILER ERROR: Cannot access member of type {:#?}",
                        other
                    )
                } else {
                    None
                }
            }
        }
    }
}

impl InferType for PostfixExpression {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        match self {
            PostfixExpression::YeetExpression(yeet) => {
                // TODO: Check if return type is matching
                match yeet.inner.infer_type(scope) {
                    Some(inner) => match inner {
                        TypeElement::Optional(res) => Some(res.some),
                        TypeElement::Result(res) => Some(res.success),
                        _ => todo!("TRANSPILER_ERROR: Yeet operator can only be used on result or optional types"),
                    },
                    None => None,
                }
            }
            PostfixExpression::AccessExpression(access) => match access.base.infer_type(scope) {
                Some(base) => match base {
                    TypeElement::Array(array) => Some(array.elements),
                    TypeElement::Dictionary(dict) => Some(dict.value),
                    TypeElement::OrderedDictionary(dict) => Some(dict.value),
                    TypeElement::Set(set) => Some(set.elements),
                    _ => todo!(
                        "TRANSPILER_ERROR: Access operator can only be used on collection types"
                    ),
                },
                None => None,
            },
        }
    }
}

impl InferType for CollectionLiteral {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        match self {
            CollectionLiteral::ArrayLiteral(array) => array.infer_type(scope),
            CollectionLiteral::DictLiteral(dict) => dict.infer_type(scope),
            CollectionLiteral::SetLiteral(set) => set.infer_type(scope),
            CollectionLiteral::OrderedDictLiteral(ordered_dict) => ordered_dict.infer_type(scope),
        }
    }
}

impl InferType for ArrayLiteral {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let elements = infer_from_elements(&self.elements, scope);
        Some(
            Box::new(ArrayTypeItem {
                elements,
                span: Span::default(),
            })
            .into(),
        )
    }
}

impl InferType for SetLiteral {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let elements = infer_from_elements(&self.elements, scope);
        Some(
            Box::new(SetTypeItem {
                elements,
                span: Span::default(),
            })
            .into(),
        )
    }
}

impl InferType for DictLiteral {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let (key, value) = infer_dict_elements(&self.elements, scope);
        Some(
            Box::new(DictionaryTypeItem {
                key,
                value,
                span: Span::default(),
            })
            .into(),
        )
    }
}

impl InferType for OrderedDictLiteral {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let (key, value) = infer_dict_elements(&self.elements, scope);
        Some(
            Box::new(OrderedDictionaryTypeItem {
                key,
                value,
                span: Span::default(),
            })
            .into(),
        )
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
        .filter_map(|item| item.infer_type(scope))
        .unique()
        .collect::<Vec<_>>();

    match inner.len() {
        0 => infer(),
        1 => inner.into_iter().next().unwrap(),
        _ => todo!("TRANSPILE ERROR: Cannot infer type of array literal with multiple types"),
    }
}
