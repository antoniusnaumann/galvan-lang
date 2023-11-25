use derive_more::From;

use galvan_pest::Rule;

use crate::TypeIdent;

#[derive(Debug, PartialEq, Eq, From, FromPest)]
#[pest_ast(rule(Rule::type_item))]
pub enum TypeElement {
    // Collection Types
    Array(Box<ArrayTypeItem>),
    Dictionary(Box<DictionaryTypeItem>),
    OrderedDictionary(Box<OrderedDictionaryTypeItem>),
    Set(Box<SetTypeItem>),
    Tuple(Box<TupleTypeItem>),

    // Error handling monads
    Optional(Box<OptionalTypeItem>),
    Result(Box<ResultTypeItem>),

    // Primitive type
    Plain(BasicTypeItem),
}

impl From<TypeIdent> for TypeElement {
    fn from(value: TypeIdent) -> Self {
        Self::Plain(BasicTypeItem { ident: value })
    }
}

impl TypeElement {
    pub fn plain(ident: TypeIdent) -> Self {
        Self::Plain(BasicTypeItem { ident })
    }

    pub fn array(elements: TypeElement) -> Self {
        Self::Array(Box::new(ArrayTypeItem { elements }))
    }

    pub fn dict(key: TypeElement, value: TypeElement) -> Self {
        Self::Dictionary(Box::new(DictionaryTypeItem { key, value }))
    }

    pub fn ordered_dict(key: TypeElement, value: TypeElement) -> Self {
        Self::OrderedDictionary(Box::new(OrderedDictionaryTypeItem { key, value }))
    }

    pub fn set(elements: TypeElement) -> Self {
        Self::Set(Box::new(SetTypeItem { elements }))
    }

    pub fn tuple(elements: Vec<TypeElement>) -> Self {
        Self::Tuple(Box::new(TupleTypeItem { elements }))
    }

    pub fn optional(some: OptionalElement) -> Self {
        Self::Optional(Box::new( OptionalTypeItem { some }))
    }

    pub fn result(success: TypeElement) -> Self {
        Self::Result(Box::new(ResultTypeItem {
            success,
            error: None,
        }))
    }

    pub fn result_with_typed_error(success: TypeElement, error: TypeElement) -> Self {
        Self::Result(Box::new(ResultTypeItem {
            success,
            error: Some(error),
        }))
    }
}

// TODO: Add a marker trait to constrain this to only type decls
#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::array_type))]
pub struct ArrayTypeItem {
    pub elements: TypeElement,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::dict_type))]
pub struct DictionaryTypeItem {
    pub key: TypeElement,
    pub value: TypeElement,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::ordered_dict_type))]
pub struct OrderedDictionaryTypeItem {
    pub key: TypeElement,
    pub value: TypeElement,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::set_type))]
pub struct SetTypeItem {
    pub elements: TypeElement,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::tuple_type))]
pub struct TupleTypeItem {
    pub elements: Vec<TypeElement>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::optional_type))]
pub struct OptionalTypeItem {
    pub some: OptionalElement,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::opt_element_type))]
pub enum OptionalElement {
    Array(Box<ArrayTypeItem>),
    Dictionary(Box<DictionaryTypeItem>),
    OrderedDictionary(Box<OrderedDictionaryTypeItem>),
    Set(Box<SetTypeItem>),
    Tuple(Box<TupleTypeItem>),
    Plain(Box<BasicTypeItem>),
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::result_type))]
pub struct ResultTypeItem {
    pub success: TypeElement,
    pub error: Option<TypeElement>,
}

#[derive(Debug, PartialEq, Eq, FromPest)]
#[pest_ast(rule(Rule::basic_type))]
pub struct BasicTypeItem {
    pub ident: TypeIdent,
    // TODO: Handle generics
}

#[cfg(test)]
mod test {
    use from_pest::FromPest;
    use from_pest::pest::Parser;
    use galvan_pest::Rule;
    use crate::*;

    fn partial_ast<'p, T>(src: &'p str, rule: Rule) -> Result<T, String>
        where
            T: FromPest<'p, Rule = Rule>,
    {
        let pairs = galvan_pest::GalvanParser::parse(rule, src).unwrap();
        T::from_pest(&mut pairs.clone()).map_err(|_| format!("Error when converting into ast!\n\n{pairs:#?}"))
    }

    #[test]
    fn test_plain_type() {
        let parsed: TypeElement = partial_ast("Int", Rule::type_item).unwrap_or_else(|e| panic!("{}", e));
        let TypeElement::Plain(basic) = parsed else { panic!("Expected plain type") };
        assert_eq!(basic.ident, TypeIdent::new("Int"));
    }

    macro_rules! test_collection_type {
        ($lit:literal, $name:ident, $rule:ident, $variant:ident, $inner:ident) => {
            #[test]
            fn $name() {
                let parsed: $inner = partial_ast($lit, Rule::$rule).unwrap_or_else(|e| panic!("{}", e));
                let TypeElement::Plain(elements) = parsed.elements else { panic!("Expected plain type as element type") };
                assert_eq!(elements.ident, TypeIdent::new("Int"), "Tested {} type", stringify!($inner));

                let parsed: TypeElement = partial_ast($lit, Rule::type_item).unwrap_or_else(|e| panic!("{}", e));
                let TypeElement::$variant(container) = parsed else { panic!("Wrong type") };
                let TypeElement::Plain(elements) = container.elements else { panic!("Expected plain type") };
                assert_eq!(elements.ident, TypeIdent::new("Int"), "Tested TypeItem");
            }
        };
    }

    test_collection_type!("[Int]", test_array_type, array_type, Array, ArrayTypeItem);
    test_collection_type!("{Int}", test_set_type, set_type, Set, SetTypeItem);

    #[test]
    fn test_tuple_type() {
        let parsed: TupleTypeItem = partial_ast("(Int, Float)", Rule::tuple_type).unwrap_or_else(|e| panic!("{}", e));
        let TypeElement::Plain(ref elements) = parsed.elements[0] else { panic!("Expected plain type as first element!") };
        assert_eq!(elements.ident, TypeIdent::new("Int"), "Testing first element");
        let TypeElement::Plain(ref elements) = parsed.elements[1] else { panic!("Expected plain type as second element!") };
        assert_eq!(elements.ident, TypeIdent::new("Float"), "Testing second element");

        let parsed: TypeElement = partial_ast("(Int, String)", Rule::type_item).unwrap_or_else(|e| panic!("{}", e));
        let TypeElement::Tuple(container) = parsed else { panic!("Wrong type") };
        let TypeElement::Plain(ref elements) = container.elements[0] else { panic!("Expected plain type as first element!") };
        assert_eq!(elements.ident, TypeIdent::new("Int"), "Testing first element for lifted TypeItem");
        let TypeElement::Plain(ref elements) = container.elements[1] else { panic!("Expected plain type as second element!") };
        assert_eq!(elements.ident, TypeIdent::new("String"), "Testing second element for lifted TypeItem");
    }

    macro_rules! test_dictionary_type {
        ($lit:literal, $name:ident, $rule:ident, $variant:ident, $inner:ident) => {
            #[test]
            fn $name() {
                let parsed: $inner = partial_ast($lit, Rule::$rule).unwrap_or_else(|e| panic!("{}", e));
                let TypeElement::Plain(key) = parsed.key else { panic!("Expected plain type as key type") };
                assert_eq!(key.ident, TypeIdent::new("Int"), "Testing key");
                let TypeElement::Plain(value) = parsed.value else { panic!("Expected plain type as value type") };
                assert_eq!(value.ident, TypeIdent::new("Float"), "Testing value");

                let parsed: TypeElement = partial_ast($lit, Rule::type_item).unwrap_or_else(|e| panic!("{}", e));
                let TypeElement::$variant(container) = parsed else { panic!("Wrong type") };
                let TypeElement::Plain(key) = container.key else { panic!("Expected plain type as key type") };
                assert_eq!(key.ident, TypeIdent::new("Int"), "Testing key for lifted TypeItem");
                let TypeElement::Plain(value) = container.value else { panic!("Expected plain type as value type") };
                assert_eq!(value.ident, TypeIdent::new("Float"), "Testing value for lifted TypeItem");
            }
        };
    }

    test_dictionary_type!("{Int: Float}", test_dict_type, dict_type, Dictionary, DictionaryTypeItem);
    test_dictionary_type!("[Int: Float]", test_ordered_dict_type, ordered_dict_type, OrderedDictionary, OrderedDictionaryTypeItem);
}