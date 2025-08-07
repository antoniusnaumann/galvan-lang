use galvan_ast::{
    ArrayTypeItem, BasicTypeItem, DictionaryTypeItem, GenericTypeItem, Ident, OptionalTypeItem,
    OrderedDictionaryTypeItem, ResultTypeItem, SetTypeItem, Span, TupleTypeItem, TypeElement,
    TypeIdent,
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for TypeElement {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let type_item = cursor_expect!(cursor, "type_item");
        let _span = Span::from_node(type_item);

        cursor.child();

        let inner = match cursor.kind()? {
            "result_type" => {
                TypeElement::Result(ResultTypeItem::read_cursor(cursor, source)?.into())
            }
            "optional_type" => {
                TypeElement::Optional(OptionalTypeItem::read_cursor(cursor, source)?.into())
            }
            "array_type" => TypeElement::Array(ArrayTypeItem::read_cursor(cursor, source)?.into()),
            "dict_type" => {
                TypeElement::Dictionary(DictionaryTypeItem::read_cursor(cursor, source)?.into())
            }
            "ordered_dict_type" => TypeElement::OrderedDictionary(
                OrderedDictionaryTypeItem::read_cursor(cursor, source)?.into(),
            ),
            "set_type" => TypeElement::Set(SetTypeItem::read_cursor(cursor, source)?.into()),
            "tuple_type" => TypeElement::Tuple(TupleTypeItem::read_cursor(cursor, source)?.into()),
            "generic_type" => TypeElement::Generic(GenericTypeItem::read_cursor(cursor, source)?),
            "basic_type" => TypeElement::Plain(BasicTypeItem::read_cursor(cursor, source)?),
            unknown => {
                unimplemented!("Encountered type element not known to AST converstion: {unknown}")
            }
        };

        // TODO  Verify that there is no other child node

        cursor.goto_parent();

        Ok(inner)
    }
}

impl ReadCursor for ResultTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "result_type");
        let span = Span::from_node(node);

        cursor.child();
        let success = TypeElement::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "exclamation_mark");

        cursor.next();
        let error = if cursor.kind()? == "type_item" {
            Some(TypeElement::read_cursor(cursor, source)?)
        } else {
            None
        };

        cursor.goto_parent();
        Ok(ResultTypeItem {
            success,
            error,
            span,
        })
    }
}

impl ReadCursor for OptionalTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let optional = cursor_expect!(cursor, "optional_type");
        let span = Span::from_node(optional);

        cursor.child();
        let some = TypeElement::read_cursor(cursor, source)?;

        cursor.next();
        let _question_mark = cursor_expect!(cursor, "question_mark");

        cursor.goto_parent();

        Ok(Self { inner: some, span })
    }
}

impl ReadCursor for ArrayTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let array = cursor_expect!(cursor, "array_type");
        let span = Span::from_node(array);

        cursor.child();
        let _bracket = cursor_expect!(cursor, "bracket_open");

        cursor.next();
        let inner = TypeElement::read_cursor(cursor, source)?;

        cursor.next();
        let _bracket = cursor_expect!(cursor, "bracket_close");

        cursor.goto_parent();

        Ok(Self {
            elements: inner,
            span,
        })
    }
}

impl ReadCursor for DictionaryTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let dict = cursor_expect!(cursor, "dict_type");
        let span = Span::from_node(dict);

        cursor.child();
        let _brace = cursor_expect!(cursor, "brace_open");

        cursor.next();
        let key = TypeElement::read_cursor(cursor, source)?;

        cursor.next();
        let _colon = cursor_expect!(cursor, "colon");

        cursor.next();
        let value = TypeElement::read_cursor(cursor, source)?;

        cursor.next();
        let _brace = cursor_expect!(cursor, "brace_close");

        cursor.goto_parent();

        Ok(Self { key, value, span })
    }
}

impl ReadCursor for OrderedDictionaryTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let dict = cursor_expect!(cursor, "ordered_dict_type");
        let span = Span::from_node(dict);

        cursor.child();
        let _bracket = cursor_expect!(cursor, "bracket_open");

        cursor.next();
        let key = TypeElement::read_cursor(cursor, source)?;

        cursor.next();
        let _colon = cursor_expect!(cursor, "colon");

        cursor.next();
        let value = TypeElement::read_cursor(cursor, source)?;

        cursor.next();
        let _bracket = cursor_expect!(cursor, "bracket_close");

        cursor.goto_parent();

        Ok(Self { key, value, span })
    }
}

impl ReadCursor for SetTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let set = cursor_expect!(cursor, "set_type");
        let span = Span::from_node(set);

        cursor.child();
        let _brace = cursor_expect!(cursor, "brace_close");

        cursor.next();
        let inner = TypeElement::read_cursor(cursor, source)?;

        cursor.next();
        let _brace = cursor_expect!(cursor, "brace_close");

        cursor.goto_parent();

        Ok(Self {
            elements: inner,
            span,
        })
    }
}

impl ReadCursor for TupleTypeItem {
    fn read_cursor(_cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for GenericTypeItem {
    fn read_cursor(_cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for BasicTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let type_item = cursor_expect!(cursor, "basic_type");
        let span = Span::from_node(type_item);

        cursor.child();

        let ident = TypeIdent::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(Self { ident, span })
    }
}

impl ReadCursor for TypeIdent {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let ident = cursor_expect!(cursor, "type_ident");
        let inner = &source[ident.start_byte()..ident.end_byte()];

        Ok(Self::new(inner))
    }
}

impl ReadCursor for Ident {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let ident = cursor_expect!(cursor, "ident");
        let inner = &source[ident.start_byte()..ident.end_byte()];

        Ok(Self::new(inner))
    }
}
