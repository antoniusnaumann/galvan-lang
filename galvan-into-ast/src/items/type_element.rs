use std::intrinsics::unreachable;

use galvan_ast::{BasicTypeItem, TypeElement};
use galvan_parse::TreeCursor;

use crate::{result::CursorUtil, AstError, ReadCursor};

impl ReadCursor for TypeElement {
    fn read_cursor(cursor: &mut TreeCursor<'_>) -> Result<Self, AstError> {
        let type_item = cursor_expect!(cursor, "type_item");
        let span = Span::from_node(type_item);
        
        cursor.goto_first_child();
        
        let inner = match cursor.kind() {
            "result_type" => TypeElement::Result(()),
            "optional_type" => TypeElement::Optional(()),
            "array_type" => TypeElement::Array(()),
            "dict_type" => TypeElement::Dictionary(()),
            "ordered_dict_type" => TypeElement::OrderedDictionary(()),
            "set_type" => TypeElement::Set(()),
            "tuple_type" => TypeElement::Tuple(()),
            "generic_type" => TypeElement::Generic(()),
            "basic_type" => TypeElement::Plain(BasicTypeItem::read_cursor(cursor)),
            unknown => unimplemented!("Encountered type element not known to AST converstion: {unknown}"),
        }

        // TODO  Verify that there is no other child node

        cursor.goto_parent()

        Ok(inner)
    }
}

impl ReadCursor for BasicTypeItem {
}
