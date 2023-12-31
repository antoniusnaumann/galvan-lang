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

    pub fn optional(ty: TypeElement) -> TypeElement {
        TypeElement::optional(ty.try_into().unwrap())
    }

    pub fn result(success: TypeElement, error: Option<TypeElement>) -> TypeElement {
        TypeElement::result(success, error)
    }

    pub fn array(elements: TypeElement) -> TypeElement {
        TypeElement::array(elements)
    }

    pub fn dict(key: TypeElement, value: TypeElement) -> TypeElement {
        TypeElement::dict(key, value)
    }

    pub fn struct_member(ident: &str, ty: TypeElement) -> StructTypeMember {
        StructTypeMember {
            decl_modifier: DeclModifier::Inherited,
            ident: Ident::new(ident),
            r#type: ty,
        }
    }

    pub fn ref_struct_member(ident: &str, ty: TypeElement) -> StructTypeMember {
        StructTypeMember {
            decl_modifier: DeclModifier::Ref,
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

    pub fn params(params: Vec<(DeclModifier, &'static str, TypeElement)>) -> ParamList {
        ParamList {
            params: params
                .into_iter()
                .map(|(decl_modifier, name, ty)| Param {
                    decl_modifier,
                    identifier: Ident::new(name),
                    param_type: ty,
                })
                .collect(),
        }
    }

    pub fn empty_body() -> Block {
        Block { statements: vec![] }
    }

    pub fn body(statements: Vec<Statement>) -> Block {
        Block { statements }
    }

    pub fn number(value: &str) -> Expression {
        NumberLiteral::new(value).into()
    }

    pub fn variable(ident: &str) -> Expression {
        Ident::new(ident).into()
    }

    pub fn function_call(ident: &str, arguments: Vec<FunctionCallArg>) -> Expression {
        FunctionCall {
            identifier: Ident::new(ident),
            arguments,
        }
        .into()
    }

    pub fn ident_arg(modifier: DeclModifier, idents: &[&str]) -> FunctionCallArg {
        FunctionCallArg::Ident(IdentArg {
            modifier,
            dotted: idents.iter().map(|&ident| Ident::new(ident)).collect(),
        })
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
