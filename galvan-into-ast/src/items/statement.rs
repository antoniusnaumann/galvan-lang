use crate::items::{read_free_function_call, read_trailing_closure_call};
use crate::result::CursorUtil;
use crate::{cursor_expect, AstError, ReadCursor, SpanExt};
use galvan_ast::{
    Assignment, Closure, CollectionLiteral, ConstructorCall, DeclModifier, Declaration, ElseExpression, Expression, FunctionCall, Group, Ident, InfixExpression, Literal, PostfixExpression, Span, Statement, TypeElement
};
use galvan_parse::TreeCursor;

impl ReadCursor for Statement {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "statement");

        cursor.goto_first_child();
        let inner = match cursor.kind()? {
            "assignment" => Statement::Assignment(Assignment::read_cursor(cursor, source)?),
            "declaration" => Statement::Declaration(Declaration::read_cursor(cursor, source)?),
            "expression" => Statement::Expression(Expression::read_cursor(cursor, source)?),
            "free_function" => {
                Statement::Expression(read_free_function_call(cursor, source)?.into())
            }
            _ => unreachable!("Unknown statement kind: {:?}", cursor.kind()?),
        };

        cursor.goto_parent();
        Ok(inner)
    }
}

impl ReadCursor for Declaration {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "declaration");
        let span = Span::from_node(node);

        cursor.goto_first_child();

        let decl_modifier = DeclModifier::read_cursor(cursor, source)?;
        cursor.goto_next_sibling();
        let identifier = Ident::read_cursor(cursor, source)?;
        cursor.goto_next_sibling();

        let type_annotation = match cursor.kind()? {
            "colon" => {
                cursor.goto_next_sibling();
                Some(TypeElement::read_cursor(cursor, source)?)
            }
            _ => None,
        };

        let assignment = match cursor.kind()? {
            "assign" => {
                cursor.goto_next_sibling();
                Some(Expression::read_cursor(cursor, source)?)
            }
            _ => None,
        };

        cursor.goto_parent();

        let decl = Declaration { decl_modifier, identifier, type_annotation, assignment, span };

        Ok(decl)
    }
}

impl ReadCursor for Assignment {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!("Implement Assignment::read_cursor")
    }
}

impl ReadCursor for Expression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "expression");

        cursor.goto_first_child();
        
        let inner: Expression = match cursor.kind()? {
            "else_expression" => ElseExpression::read_cursor(cursor, source)?.into(),
            "trailing_closure_expression" => read_trailing_closure_call(cursor, source)?.into(),   
            "function_call" => FunctionCall::read_cursor(cursor, source)?.into(),
            "postfix_expression" => Expression::Postfix(PostfixExpression::read_cursor(cursor, source)?.into()),
            "constructor_call" => ConstructorCall::read_cursor(cursor, source)?.into(),
             "collection_literal" => CollectionLiteral::read_cursor(cursor, source)?.into(),
            "literal" => Literal::read_cursor(cursor, source)?.into(),
            "ident" => Ident::read_cursor(cursor, source)?.into(),
            "closure" => Closure::read_cursor(cursor, source)?.into(),
            "group" => Group::read_cursor(cursor, source)?.into(),
            _ => Expression::Infix(InfixExpression::read_cursor(cursor, source)?.into()),
        };
        
        cursor.goto_parent();

        Ok(inner)
    }
}
