use galvan_ast::{
    ArrayLiteral, ArrayTypeItem, BasicTypeItem, Block, Body, CollectionLiteral, CollectionOperator,
    DictLiteral, DictLiteralElement, DictionaryTypeItem, ElseExpression, Expression, InfixOperator,
    Literal, MemberChain, OperatorTree, OperatorTreeNode, OrderedDictLiteral,
    OrderedDictionaryTypeItem, SetLiteral, SetTypeItem, SimpleExpression, SingleExpression,
    Statement, TopExpression, TypeDecl, TypeElement, TypeIdent,
};
use galvan_resolver::{Lookup, Scope};
use itertools::Itertools;

pub(crate) trait InferType {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement>;
}

impl InferType for TopExpression {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        match self {
            TopExpression::Expression(e) => e.infer_type(scope),
            TopExpression::ElseExpression(e) => e.infer_type(scope),
        }
    }
}

impl InferType for ElseExpression {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let receiver_type = self.receiver.infer_type(scope);
        let block_type = self.block.infer_type(scope);

        match (receiver_type, block_type) {
            (Some(receiver_type), Some(block_type)) => {
                if receiver_type == block_type {
                    Some(receiver_type)
                } else {
                    todo!("TRANSPILER ERROR: Types of if and else expression don't match. (allow this when type unions are implemented)")
                }
            }
            (Some(receiver_type), None) => Some(receiver_type),
            (None, Some(block_type)) => Some(block_type),
            (None, None) => None,
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
            Statement::TopExpression(expr) => expr.infer_type(scope),
            Statement::Declaration(_) => None,
            Statement::Block(block) => block.infer_type(scope),
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
            Expression::OperatorTree(tree) => tree.infer_type(scope),
            Expression::MemberChain(access) => access.infer_type(scope),
            Expression::SingleExpression(s) => s.infer_type(scope),
        }
    }
}

impl InferType for SimpleExpression {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        match self {
            SimpleExpression::MemberChain(access) => access.infer_type(scope),
            SimpleExpression::SingleExpression(expr) => expr.infer_type(scope),
        }
    }
}

impl InferType for SingleExpression {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        match self {
            SingleExpression::CollectionLiteral(collection) => collection.infer_type(scope),
            SingleExpression::FunctionCall(call) => {
                // todo!("Implement type inference for function call")
                None
            }
            SingleExpression::ConstructorCall(constructor) => {
                Some(constructor.identifier.clone().into())
            }
            SingleExpression::Literal(literal) => literal.infer_type(scope),
            SingleExpression::Ident(ident) => scope.get_variable(ident)?.ty.clone()?.into(),
            SingleExpression::Postfix(_) => todo!(),
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
                }
                .into(),
            ),
            Literal::NumberLiteral(_) => Some(
                BasicTypeItem {
                    ident: TypeIdent::new("__Number"),
                }
                .into(),
            ),
        }
    }
}

impl InferType for OperatorTree {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let Self {
            left,
            operator,
            right,
        } = self;

        match operator {
            InfixOperator::Arithmetic(op) => {
                // todo!("Implement type inference for arithmetic operator")
                None
            }
            InfixOperator::Collection(op) => infer_collection_operation(scope, *op, left, right),
            InfixOperator::Comparison(_) => Some(bool()),
            InfixOperator::Logical(_) => Some(bool()),
            InfixOperator::CustomInfix(op) => {
                // todo!("Implement type inference for custom infix operator")
                None
            }
        }
    }
}

fn infer_collection_operation(
    scope: &Scope,
    op: CollectionOperator,
    lhs: &OperatorTreeNode,
    rhs: &OperatorTreeNode,
) -> Option<TypeElement> {
    match op {
        CollectionOperator::Concat => {
            // todo!("Implement type inference for collection concat")
            None
        }
        CollectionOperator::Remove => {
            // todo!("Implement type inference for collection remove")
            None
        }
        CollectionOperator::Contains => Some(bool()),
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

impl InferType for MemberChain {
    fn infer_type(&self, scope: &Scope) -> Option<TypeElement> {
        let Self { elements } = self;

        let receiver_type = if elements.len() == 2 {
            elements[0].infer_type(scope)?
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
                    TypeDecl::Struct(st) => {
                        let field = self.field_ident()?;
                        st.members
                            .iter()
                            .find(|member| member.ident == *field)
                            .map(|member| member.r#type.clone())
                    }
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
