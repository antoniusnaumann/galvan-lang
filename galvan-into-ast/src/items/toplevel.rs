use galvan_ast::{
    AliasTypeDecl, Body, DeclModifier, EmptyTypeDecl, EnumTypeDecl, FnDecl, FnSignature, Ident,
    MainDecl, Param, ParamList, RootItem, Span, Statement, StringLiteral, StructTypeDecl, TestDecl,
    TupleTypeDecl, TypeDecl, TypeElement, Visibility,
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for RootItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        Ok(match cursor.kind()? {
            "main" => MainDecl::read_cursor(cursor, source)?.into(),
            "build" => todo!("Implement build entry point!"),
            "test" => TestDecl::read_cursor(cursor, source)?.into(),
            "function" => FnDecl::read_cursor(cursor, source)?.into(),
            "type_declaration" => TypeDecl::read_cursor(cursor, source)?.into(),
            "entry_point" => todo!("Implement custom tasks!"),
            other => unreachable!("Unexpected node in root item: {other}"),
        })
    }
}

impl ReadCursor for MainDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let main = cursor_expect!(cursor, "main");
        let span = Span::from_node(main);
        cursor.child();
        cursor_expect!(cursor, "main_keyword");

        cursor.next();
        let body = Body::read_cursor(cursor, source)?;

        assert!(!cursor.next(), "Unexpected token in main");

        cursor.goto_parent();

        Ok(MainDecl { body, span })
    }
}

impl ReadCursor for TestDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let test = cursor_expect!(cursor, "test");
        let span = Span::from_node(test);
        cursor.child();
        cursor_expect!(cursor, "test_keyword");

        cursor.next();
        let name = if cursor.kind()? == "string_literal" {
            let lit = Some(StringLiteral::read_cursor(cursor, source)?);
            cursor.next();
            lit
        } else {
            None
        };

        cursor.next();
        let body = Body::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(TestDecl { name, body })
    }
}

impl ReadCursor for TypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let ty = cursor_expect!(cursor, "type_declaration");
        cursor.child();

        // TODO  Parse type declaration variatns
        let decl = match cursor.kind()? {
            "struct" => StructTypeDecl::read_cursor(cursor, source)?.into(),
            "alias" => AliasTypeDecl::read_cursor(cursor, source)?.into(),
            "enum" => EnumTypeDecl::read_cursor(cursor, source)?.into(),
            "tuple_struct" => TupleTypeDecl::read_cursor(cursor, source)?.into(),
            "empty_struct" => EmptyTypeDecl::read_cursor(cursor, source)?.into(),
            _ => unreachable!(),
        };

        cursor.goto_parent();

        Ok(decl)
    }
}

impl ReadCursor for FnDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let function = cursor_expect!(cursor, "function");
        let span = Span::from_node(function);
        cursor.child();

        let signature = FnSignature::read_cursor(cursor, source)?;

        cursor.next();
        let body = Body::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(FnDecl {
            signature,
            body,
            span,
        })
    }
}

impl ReadCursor for Body {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let body = cursor_expect!(cursor, "body");
        let span = Span::from_node(body);
        cursor.child();

        cursor_expect!(cursor, "brace_open");
        cursor.next();

        let mut statements = vec![];
        while cursor.kind()? == "statement" {
            let stmt = Statement::read_cursor(cursor, source)?;
            statements.push(stmt);
            cursor.next();
        }

        cursor_expect!(cursor, "brace_close");
        cursor.goto_parent();

        Ok(Body { statements, span })
    }
}

impl ReadCursor for FnSignature {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let signature = cursor_expect!(cursor, "fn_signature");
        let span = Span::from_node(signature);
        cursor.child();

        let visibility = Visibility::read_cursor(cursor, source)?;

        cursor_expect!(cursor, "fn_keyword");

        cursor.next();
        let identifier = Ident::read_cursor(cursor, source)?;

        cursor.next();
        let parameters = ParamList::read_cursor(cursor, source)?;

        cursor.next();
        let return_type = if cursor.kind()? == "return_type" {
            cursor.child();
            cursor_expect!(cursor, "single_arrow");
            cursor.next();
            let ty = TypeElement::read_cursor(cursor, source)?;
            cursor.goto_parent();
            ty
        } else {
            TypeElement::void()
        };

        cursor.goto_parent();

        Ok(FnSignature {
            visibility,
            identifier,
            parameters,
            return_type,
            span,
        })
    }
}

impl ReadCursor for ParamList {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "param_list");
        let span = Span::from_node(node);
        cursor.child();

        cursor_expect!(cursor, "paren_open");
        cursor.next();
        let mut params = Vec::new();
        while cursor.kind()? != "paren_close" {
            params.push(Param::read_cursor(cursor, source)?);
            cursor.next();
            while cursor.kind()? == "," {
                cursor.next();
            }
        }

        cursor.goto_parent();
        Ok(ParamList { params, span })
    }
}

impl ReadCursor for Param {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "param");
        let span = Span::from_node(node);
        cursor.child();

        let decl_modifier = if cursor.kind()? == "declaration_modifier" {
            let modifier = Some(DeclModifier::read_cursor(cursor, source)?);
            cursor.next();
            modifier
        } else {
            None
        };

        let identifier = Ident::read_cursor(cursor, source)?;
        cursor.next();

        cursor_expect!(cursor, "colon");
        cursor.next();

        let param_type = TypeElement::read_cursor(cursor, source)?;

        cursor.goto_parent();
        Ok(Param {
            decl_modifier,
            identifier,
            param_type,
            span,
        })
    }
}
