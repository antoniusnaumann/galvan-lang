use galvan_ast::{
    ArrayLiteral, ArrayTypeItem, BasicTypeItem, CollectionLiteral, CollectionOperation,
    CollectionOperator, DictLiteral, DictLiteralElement, DictionaryTypeItem, Expression,
    MemberFieldAccess, OrderedDictLiteral, OrderedDictionaryTypeItem, SetLiteral, SetTypeItem,
    TypeDecl, TypeElement, TypeIdent,
};
use galvan_resolver::{Lookup, Scope};
use itertools::Itertools;

pub(crate) trait InferType {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement>;
}
impl InferType for Expression {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        match self {
            Expression::ElseExpression(_) => {
                // todo!("Implement type inference for else expression")
                None
            }
            Expression::Closure(_) => {
                // todo!("Implement type inference for closure")
                None
            }
            Expression::CollectionOperation(op) => {
                match op.operator {
                    CollectionOperator::Concat | CollectionOperator::Remove => {
                        // todo!("Implement type inference for collection concat")
                        None
                    }
                    CollectionOperator::Contains => Some(bool()),
                }
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
            Expression::MemberFieldAccess(access) => access.infer_type(scope),
            Expression::BooleanLiteral(_)
            | Expression::LogicalOperation(_)
            | Expression::ComparisonOperation(_) => Some(bool()),
            Expression::StringLiteral(_) => Some(
                BasicTypeItem {
                    ident: TypeIdent::new("String"),
                }
                .into(),
            ),
            Expression::NumberLiteral(_) => Some(
                BasicTypeItem {
                    ident: TypeIdent::new("__Number"),
                }
                .into(),
            ),
            Expression::Ident(ident) => scope.get_variable(ident)?.ty.clone()?.into(),
            Expression::CollectionLiteral(collection) => collection.infer_type(scope),
        }
    }
}

fn bool() -> TypeElement {
    BasicTypeItem {
        ident: TypeIdent::new("Bool"),
    }
    .into()
}

fn infer() -> TypeElement {
    BasicTypeItem {
        ident: TypeIdent::new("__Infer"),
    }
    .into()
}

impl InferType for MemberFieldAccess {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let Self {
            receiver,
            identifier,
        } = self;
        let receiver_type = if receiver.len() == 1 {
            receiver[0].infer_type(scope)?
        } else {
            return None;
        };

        match receiver_type {
            TypeElement::Plain(ty) => {
                let ty = &scope.resolve_type(&ty.ident)?.item;

                match ty {
                    TypeDecl::Tuple(tuple) => {
                        todo!("IMPLEMENT: Access member of tuple type")
                    }
                    TypeDecl::Struct(st) => st
                        .members
                        .iter()
                        .find(|member| member.ident == *identifier)
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
            TypeElement::Optional(_) | TypeElement::Result(_) => {
                // TODO: Handle inference for optional and result types
                // TODO: Ultimately transition to a compiler error here
                //  that tells the user to use safe-call ?. or forward-error-call !.
                None
            }
            other => {
                todo!(
                    "TRANSPILER ERROR: Cannot access member of type {:#?}",
                    other
                )
            }
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
        Some(Box::new(ArrayTypeItem { elements }).into())
    }
}

impl InferType for SetLiteral {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let elements = infer_from_elements(&self.elements, scope);
        Some(Box::new(SetTypeItem { elements }).into())
    }
}

impl InferType for DictLiteral {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let (key, value) = infer_dict_elements(&self.elements, scope);
        Some(Box::new(DictionaryTypeItem { key, value }).into())
    }
}

impl InferType for OrderedDictLiteral {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let (key, value) = infer_dict_elements(&self.elements, scope);
        Some(Box::new(OrderedDictionaryTypeItem { key, value }).into())
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
