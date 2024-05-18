use galvan_ast::{ArrayTypeItem, BasicTypeItem, DictionaryTypeItem, GenericTypeItem, OptionalTypeItem, OrderedDictionaryTypeItem, ResultTypeItem, SetTypeItem, Span, TupleTypeItem, TypeElement, TypeIdent};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for TypeElement {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let type_item = cursor_expect!(cursor, "type_item");
        let span = Span::from_node(type_item);
        
        cursor.goto_first_child();
        
        let inner = match cursor.kind()? {
            "result_type" => TypeElement::Result(ResultTypeItem::read_cursor(cursor, source)),
            "optional_type" => TypeElement::Optional(OptionalTypeItem::read_cursor(cursor, source)),
            "array_type" => TypeElement::Array(ArrayTypeItem::read_cursor(cursor, source)),
            "dict_type" => TypeElement::Dictionary(DictionaryTypeItem::read_cursor(cursor, source)),
            "ordered_dict_type" => TypeElement::OrderedDictionary(OrderedDictionaryTypeItem::read_cursor(cursor, source)),
            "set_type" => TypeElement::Set(SetTypeItem::read_cursor(cursor, source)),
            "tuple_type" => TypeElement::Tuple(TupleTypeItem::read_cursor(cursor, source)),
            "generic_type" => TypeElement::Generic(GenericTypeItem::read_cursor(cursor, source)),
            "basic_type" => TypeElement::Plain(BasicTypeItem::read_cursor(cursor, source)),
            unknown => unimplemented!("Encountered type element not known to AST converstion: {unknown}"),
        };

        // TODO  Verify that there is no other child node

        cursor.goto_parent();

        Ok(inner)
    }
}

impl ReadCursor for ResultTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for OptionalTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let optional = cursor_expect!(cursor, "optional_type");
        let span = Span::from_node(optional);

        cursor.goto_first_child();
        let some = TypeElement::read_cursor(cursor, source);

        cursor.goto_next_sibling();
        let _question_mark = cursor_expect!(cursor, "question_mark");

        cursor.goto_parent();

        Self { some, span }.into()
    }
}

impl ReadCursor for ArrayTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let array = cursor_expect!(cursor, "array_type");
        let span = Span::from(array);

        cursor.goto_first_child();
        let _bracket = cursor_expect!(cursor, "bracket_open");

        cursor.goto_next_sibling();
        let inner = TypeElement::read_cursor(cursor, source);

        cursor.goto_next_sibling();
        let _bracket = cursor_expect!(cursor, "bracket_close");

        cursor.goto_parent();

        Self { elements: inner, span }.into()
    }
}

impl ReadCursor for DictionaryTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let dict = cursor_expect!(cursor, "dict_type");
        let span = Span::from(dict);

        cursor.goto_first_child();
        let _brace = cursor_expect!(cursor, "brace_open");
        
        cursor.goto_next_sibling();
        let key = TypeElement::read_cursor(cursor, source);

        cursor.goto_next_sibling();
        let _colon = cursor_expect!(cursor, "colon");

        cursor.goto_next_sibling();
        let value = TypeElement::read_cursor(cursor, source);

        cursor.goto_next_sibling();
        let _brace = cursor_expect!(cursor, "brace_close");

        cursor.goto_parent();

        Self { key, value, span }.into()
    }
}

impl ReadCursor for OrderedDictionaryTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let dict = cursor_expect!(cursor, "ordered_dict_type");
        let span = Span::from(dict);

        cursor.goto_first_child();
        let _bracket = cursor_expect!(cursor, "bracket_open");
        
        cursor.goto_next_sibling();
        let key = TypeElement::read_cursor(cursor, source);

        cursor.goto_next_sibling();
        let _colon = cursor_expect!(cursor, "colon");

        cursor.goto_next_sibling();
        let value = TypeElement::read_cursor(cursor, source);

        cursor.goto_next_sibling();
        let _bracket = cursor_expect!(cursor, "bracket_close");

        cursor.goto_parent();

        Self { key, value, span }.into()
    }
}

impl ReadCursor for SetTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let set = cursor_expect!(cursor, "set_type");
        let span = Span::from(set);

        cursor.goto_first_child();
        let _brace = cursor_expect!(cursor, "brace_close");

        cursor.goto_next_sibling();
        let inner = TypeElement::read_cursor(cursor, source);

        cursor.goto_next_sibling();
        let _brace = cursor_expect!(cursor, "brace_close");

        cursor.goto_parent();

        Self { elements: inner, span }.into()
    }
}

impl ReadCursor for TupleTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for GenericTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for BasicTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let type_item = cursor_expect!(cursor, "basic_type");
        let span = Span::from_node(type_item);
        
        cursor.goto_first_child();

        let ident = TypeIdent::read_cursor(cursor, source);

        cursor.goto_parent();

        Self { ident, span }.into()
    }
}

impl ReadCursor for TypeIdent {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let ident = cursor_expect!(cursor, "type_ident");
        let inner = source[ident.start_byte()..=ident.end_byte()];

        Self::new(inner).into()
    }
}
