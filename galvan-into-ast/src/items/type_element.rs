use galvan_ast::{ArrayTypeItem, BasicTypeItem, DictionaryTypeItem, GenericTypeItem, OptionalTypeItem, OrderedDictionaryTypeItem, ResultTypeItem, SetTypeItem, Span, TupleTypeItem, TypeElement, TypeIdent};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor};

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
        }

        // TODO  Verify that there is no other child node

        cursor.goto_parent()

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
        todo!()
    }
}

impl ReadCursor for ArrayTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for DictionaryTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for OrderedDictionaryTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for SetTypeItem {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
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

        Ok(BasicTypeItem { ident, span })
    }
}

impl ReadCursor for TypeIdent {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let ident = cursor_expect!(cursor, "type_ident");
        let inner = 

        Ok(TypeIdent::new(inner))
    }
}
