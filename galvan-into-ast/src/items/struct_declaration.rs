use galvan_ast::{
    AliasTypeDecl, AstNode, DeclModifier, EmptyTypeDecl, EnumTypeDecl, EnumTypeMember,
    EnumVariantField, Ident, Span, StructTypeDecl, StructTypeMember, TupleTypeDecl,
    TupleTypeMember, TypeElement, TypeIdent, Visibility,
};
use galvan_parse::TreeCursor;

use crate::result::CursorUtil;
use crate::{cursor_expect, AstError, ReadCursor, SpanExt};

impl ReadCursor for StructTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let struct_decl = cursor_expect!(cursor, "struct");
        let span = Span::from_node(struct_decl);

        cursor.child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.next();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "brace_open");

        cursor.next();
        let mut members = vec![];
        while cursor.kind()? == "struct_field" {
            let field = StructTypeMember::read_cursor(cursor, source)?;
            members.push(field);

            cursor.next();
            while cursor.kind()? == "," || cursor.kind()? == ";" {
                cursor.next();
            }
        }

        cursor_expect!(cursor, "brace_close");

        cursor.goto_parent();

        Ok(StructTypeDecl {
            visibility,
            ident,
            members,
            span,
        })
    }
}

impl ReadCursor for StructTypeMember {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let struct_field = cursor_expect!(cursor, "struct_field");
        let span = Span::from_node(struct_field);

        cursor.child();

        let _visibility = Visibility::read_cursor(cursor, source)?;
        let decl_modifier = if cursor.kind()? == "declaration_modifier" {
            let modifier = Some(DeclModifier::read_cursor(cursor, source)?);
            cursor.next();
            modifier
        } else {
            None
        };

        let ident = Ident::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "colon");

        cursor.next();
        let r#type = TypeElement::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(StructTypeMember {
            decl_modifier,
            ident,
            r#type,
            span,
        })
    }
}

impl ReadCursor for EnumTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let enum_decl = cursor_expect!(cursor, "enum");
        let span = Span::from_node(enum_decl);

        cursor.child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.next();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "brace_open");

        cursor.next();
        let mut members = vec![];
        while cursor.kind()? == "enum_variant" {
            let variant = EnumTypeMember::read_cursor(cursor, source)?;
            members.push(variant);

            cursor.next();
            while cursor.kind()? == "," || cursor.kind()? == ";" {
                cursor.next();
            }
        }

        cursor_expect!(cursor, "brace_close");

        cursor.goto_parent();

        Ok(EnumTypeDecl {
            visibility,
            ident,
            members,
            span,
        })
    }
}

impl ReadCursor for TupleTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "tuple_struct");
        let span = Span::from_node(node);

        cursor.child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.next();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "paren_open");

        cursor.next();
        let mut members = vec![];
        while cursor.kind()? == "tuple_field" {
            let field = TupleTypeMember::read_cursor(cursor, source)?;
            members.push(field);

            cursor.next();
            while cursor.kind()? == "," {
                cursor.next();
            }
        }

        cursor_expect!(cursor, "paren_close");

        cursor.goto_parent();

        Ok(TupleTypeDecl {
            visibility,
            ident,
            members,
            span,
        })
    }
}

impl ReadCursor for TupleTypeMember {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "tuple_field");
        let span = Span::from_node(node);

        cursor.child();

        // TODO: Implement field visibility and declaration modifier
        let _visibility = Visibility::read_cursor(cursor, source)?;
        let _decl_modifier = if cursor.kind()? == "declaration_modifier" {
            let modifier = Some(DeclModifier::read_cursor(cursor, source)?);
            cursor.next();
            modifier
        } else {
            None
        };

        cursor.next();
        let r#type = TypeElement::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(TupleTypeMember {
            // decl_modifier,
            r#type,
            span,
        })
    }
}

impl ReadCursor for AliasTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let alias = cursor_expect!(cursor, "alias");
        let span = Span::from_node(alias);

        cursor.child();

        let visibility = Visibility::read_cursor(cursor, source)?;
        cursor_expect!(cursor, "type_keyword");

        cursor.next();

        let ident = TypeIdent::read_cursor(cursor, source)?;
        cursor.next();

        cursor_expect!(cursor, "assign");

        cursor.next();
        let r#type = TypeElement::read_cursor(cursor, source)?;
        cursor.goto_parent();

        Ok(Self {
            visibility,
            ident,
            r#type,
            span,
        })
    }
}

impl ReadCursor for EmptyTypeDecl {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let empty_type = cursor_expect!(cursor, "empty_struct");
        let span = Span::from_node(empty_type);

        cursor.child();
        let visibility = Visibility::read_cursor(cursor, source)?;

        cursor.next();
        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(EmptyTypeDecl {
            visibility,
            ident,
            span,
        })
    }
}

impl ReadCursor for EnumTypeMember {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let enum_variant = cursor_expect!(cursor, "enum_variant");
        let span = Span::from_node(enum_variant);

        cursor.child();

        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.next();
        let mut fields = vec![];

        // Check if there are associated values
        if cursor.kind()? == "paren_open" {
            cursor.next(); // Move past paren_open

            while cursor.kind()? == "enum_variant_field" {
                let field = EnumVariantField::read_cursor(cursor, source)?;
                fields.push(field);

                cursor.next();
                while cursor.kind()? == "," {
                    cursor.next();
                }
            }

            cursor_expect!(cursor, "paren_close");
        }

        cursor.goto_parent();

        Ok(EnumTypeMember {
            ident,
            fields,
            span,
        })
    }
}

impl ReadCursor for EnumVariantField {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let field_node = cursor_expect!(cursor, "enum_variant_field");
        let span = Span::from_node(field_node);

        cursor.child();

        // Check if this is a named field (has ident:type) or anonymous field (just type)
        let (name, r#type) = if cursor.kind()? == "ident" {
            let ident = Ident::read_cursor(cursor, source)?;
            cursor.next();
            cursor_expect!(cursor, "colon");
            cursor.next();
            let type_element = TypeElement::read_cursor(cursor, source)?;
            (Some(ident), type_element)
        } else {
            let type_element = TypeElement::read_cursor(cursor, source)?;
            (None, type_element)
        };

        cursor.goto_parent();

        Ok(EnumVariantField { name, r#type, span })
    }
}
