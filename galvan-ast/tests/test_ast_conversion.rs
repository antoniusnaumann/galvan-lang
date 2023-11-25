use galvan_test_macro::generate_code_tests;

use galvan_ast::*;
use galvan_pest::*;

mod test_utils {
    use super::*;
    use galvan_ast::RootItem;

    pub fn ast(item: impl Into<RootItem>) -> Ast {
        Ast::new(vec![item.into()])
    }

    pub fn items(items: Vec<RootItem>) -> Ast {
        Ast::new(items)
    }

    pub fn struct_type(visibility: Visibility, ident: &str, members: Vec<StructTypeMember>) -> TypeDecl {
        TypeDecl::Struct(StructTypeDecl {
            visibility,
            ident: TypeIdent::new(ident),
            members,
        })
    }

    pub fn tuple_type(visibility: Visibility, ident: &str, members: Vec<TupleTypeMember>) -> TypeDecl {
        TypeDecl::Tuple(TupleTypeDecl {
            visibility,
            ident: TypeIdent::new(ident),
            members,
        })
    }

    pub fn alias_type(visibility: Visibility, ident: &str, ty: TypeItem) -> TypeDecl {
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

    pub fn plain(ident: &str) -> TypeItem {
        TypeItem::plain(TypeIdent::new(ident))
    }

    pub fn array(elements: TypeItem) -> TypeItem {
        TypeItem::array(elements)
    }

    pub fn struct_member(ident: &str, ty: TypeItem) -> StructTypeMember {
        StructTypeMember {
            ident: Ident::new(ident),
            r#type: ty,
        }
    }

    pub fn tuple_member(ty: TypeItem) -> TupleTypeMember {
        TupleTypeMember {
            r#type: ty,
        }
    }

    pub fn inherited() -> Visibility {
        Visibility::Inherited
    }

    pub fn public() -> Visibility {
        Visibility::public()
    }
}

use test_utils::*;

generate_code_tests!(test_ast_conversion, AST, {
    let source = Source::from_string(code);
    let parsed = parse_source(&source).unwrap();
    parsed.clone().try_into_ast().unwrap_or_else(|e| panic!("Error: {e}\nParsed:\n{:#?}", parsed))
});