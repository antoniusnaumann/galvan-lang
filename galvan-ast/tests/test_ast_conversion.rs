use galvan_test_macro::generate_code_tests;

use galvan_ast::*;

mod test_utils {
    use super::*;
    use galvan_ast::pest_adapter::*;
    use galvan_ast::RootItem;

    pub fn empty() -> PestAst {
        PestAst::new(vec![])
    }

    pub fn single(item: impl Into<RootItem>) -> PestAst {
        PestAst::new(vec![item.into()])
    }

    pub fn multi(items: Vec<RootItem>) -> PestAst {
        PestAst::new(items)
    }

    pub fn struct_type(
        visibility: Visibility,
        ident: &str,
        members: Vec<StructTypeMember>,
    ) -> TypeDecl {
        TypeDecl::Struct(StructTypeDecl {
            visibility,
            ident: TypeIdent::new(ident),
            members,
        })
    }

    pub fn tuple_type(
        visibility: Visibility,
        ident: &str,
        members: Vec<TupleTypeMember>,
    ) -> TypeDecl {
        TypeDecl::Tuple(TupleTypeDecl {
            visibility,
            ident: TypeIdent::new(ident),
            members,
        })
    }

    pub fn alias_type(visibility: Visibility, ident: &str, ty: TypeElement) -> TypeDecl {
        TypeDecl::Alias(AliasTypeDecl {
            visibility,
            ident: TypeIdent::new(ident),
            r#type: ty,
        })
    }

    pub fn empty_type(visibility: Visibility, ident: &str) -> TypeDecl {
        TypeDecl::Empty(EmptyTypeDecl {
            visibility,
            ident: TypeIdent::new(ident),
        })
    }

    pub fn plain(ident: &str) -> TypeElement {
        TypeElement::plain(TypeIdent::new(ident))
    }

    pub fn ref_type(ty: TypeElement) -> TypeElement {
        let element = match ty {
            TypeElement::Array(elem) => RefElement::Array(elem),
            TypeElement::Dictionary(elem) => RefElement::Dictionary(elem),
            TypeElement::OrderedDictionary(elem) => RefElement::OrderedDictionary(elem),
            TypeElement::Set(elem) => RefElement::Set(elem),
            TypeElement::Tuple(elem) => RefElement::Tuple(elem),
            TypeElement::Optional(_) => panic!("Ref to optional is not allowed"),
            TypeElement::Result(_) => panic!("Ref to result is not allowed"),
            TypeElement::Ref(_) => panic!("Ref to ref is not allowed"),
            TypeElement::Plain(elem) => RefElement::Plain(elem),
        };

        TypeElement::Ref(Box::from(RefTypeItem { element }))
    }

    pub fn optional(ty: TypeElement) -> TypeElement {
        TypeElement::optional(ty.try_into().unwrap())
    }

    pub fn result(success: SuccessVariant, error: Option<ErrorVariant>) -> TypeElement {
        TypeElement::result(success, error)
    }

    pub fn success(ty: TypeElement) -> SuccessVariant {
        match ty {
            TypeElement::Plain(ident) => SuccessVariant::Plain(ident),
            TypeElement::Array(elements) => SuccessVariant::Array(elements),
            TypeElement::Dictionary(dict) => SuccessVariant::Dictionary(dict),
            TypeElement::OrderedDictionary(dict) => SuccessVariant::OrderedDictionary(dict),
            TypeElement::Set(elements) => SuccessVariant::Set(elements),
            TypeElement::Tuple(elements) => SuccessVariant::Tuple(elements),
            TypeElement::Optional(element) => SuccessVariant::Optional(element),
            TypeElement::Result(_) => panic!("Result type cannot be a success variant"),
            TypeElement::Ref(element) => SuccessVariant::Ref(element),
        }
    }

    pub fn error(ty: TypeElement) -> Option<ErrorVariant> {
        Some(match ty {
            TypeElement::Plain(ident) => ErrorVariant::Plain(ident),
            TypeElement::Array(elements) => ErrorVariant::Array(elements),
            TypeElement::Dictionary(dict) => ErrorVariant::Dictionary(dict),
            TypeElement::OrderedDictionary(dict) => ErrorVariant::OrderedDictionary(dict),
            TypeElement::Set(elements) => ErrorVariant::Set(elements),
            TypeElement::Tuple(elements) => ErrorVariant::Tuple(elements),
            TypeElement::Optional(_) => panic!("Optional type cannot be an error variant"),
            TypeElement::Result(_) => panic!("Result type cannot be an error variant"),
            TypeElement::Ref(_) => panic!("Stored reference cannot be an error variant"),
        })
    }

    pub fn array(elements: TypeElement) -> TypeElement {
        TypeElement::array(elements)
    }

    pub fn dict(key: TypeElement, value: TypeElement) -> TypeElement {
        TypeElement::dict(key, value)
    }

    pub fn struct_member(ident: &str, ty: TypeElement) -> StructTypeMember {
        StructTypeMember {
            ident: Ident::new(ident),
            r#type: ty,
        }
    }

    pub fn tuple_member(ty: TypeElement) -> TupleTypeMember {
        TupleTypeMember { r#type: ty }
    }

    pub fn inherited() -> Visibility {
        Visibility::Inherited
    }

    pub fn public() -> Visibility {
        Visibility::public()
    }

    pub fn function(
        visibility: Visibility,
        name: &str,
        parameters: ParamList,
        return_type: Option<TypeElement>,
        block: Block,
    ) -> FnDecl {
        FnDecl {
            signature: FnSignature {
                visibility,
                identifier: Ident::new(name),
                parameters,
                return_type,
            },
            block,
        }
    }

    pub fn params(params: Vec<(&'static str, TypeElement)>) -> ParamList {
        ParamList {
            params: params
                .into_iter()
                .map(|(name, ty)| Param {
                    identifier: Ident::new(name),
                    param_type: ty,
                })
                .collect(),
        }
    }

    pub fn body() -> Block {
        Block { statements: vec![] }
    }
}

use galvan_ast::pest_adapter::*;
#[allow(unused_imports)]
use galvan_files::Source;
#[allow(unused_imports)]
use galvan_pest::parse_source;
#[allow(unused_imports)]
use test_utils::*;

generate_code_tests!(test_ast_conversion, AST, {
    let source = Source::from_string(code);
    let parsed = parse_source(&source).unwrap();
    parsed
        .clone()
        .try_into_ast()
        .unwrap_or_else(|e| panic!("Error: {e}\nParsed:\n{:#?}", parsed))
});
