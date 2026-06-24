use galvan_ast::{
    Assignment, AssignmentOperator, Closure, CollectionLiteral, ConstructorCall, DeclModifier,
    Declaration, ElseExpression, EnumAccess, EnumConstructor, Expression, ExpressionKind,
    FunctionCall, Group, Ident, InfixExpression, Literal, MatchExpression, ModifiedExpression,
    PostfixExpression, Span, Statement, TypeElement,
};
use galvan_parse::TreeCursor;

use crate::items::{read_free_function_call, read_trailing_closure_call};
use crate::result::CursorUtil;
use crate::{cursor_expect, AstError, ReadCursor, SpanExt};

impl ReadCursor for Statement {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "statement");

        cursor.child();
        let inner = match cursor.kind()? {
            "assignment" => Statement::Assignment(Assignment::read_cursor(cursor, source)?),
            "declaration" => Statement::Declaration(Declaration::read_cursor(cursor, source)?),
            "expression" => Statement::Expression(Expression::read_cursor(cursor, source)?),
            "free_function" => read_free_function_call(cursor, source)?,
            "break_statement" => {
                let node = cursor.node();
                let span = Span::from_node(node);
                Statement::Break(galvan_ast::Break { span })
            }
            "continue_statement" => {
                let node = cursor.node();
                let span = Span::from_node(node);
                Statement::Continue(galvan_ast::Continue { span })
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

        cursor.child();

        let decl_modifier = DeclModifier::read_cursor(cursor, source)?;
        cursor.next();
        let identifier = Ident::read_cursor(cursor, source)?;
        cursor.next();

        let type_annotation = match cursor.kind()? {
            "colon" => {
                cursor.next();
                let elem = Some(TypeElement::read_cursor(cursor, source)?);
                cursor.next();
                elem
            }
            _ => None,
        };

        let (assignment_modifier, assignment) = match cursor.kind()? {
            "assign" => {
                cursor.next();
                let modifier = read_optional_expression_modifier(cursor, source)?;
                let expr = Some(Expression::read_cursor(cursor, source)?);
                cursor.next();
                (modifier, expr)
            }
            _ => (None, None),
        };

        cursor.goto_parent();

        let decl = Declaration {
            decl_modifier,
            identifier,
            type_annotation,
            assignment_modifier,
            assignment,
            span,
        };

        Ok(decl)
    }
}

impl ReadCursor for Assignment {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "assignment");
        let span = Span::from_node(node);

        cursor.child();
        let lhs = Expression::read_cursor(cursor, source)?;

        cursor.next();
        let operator = AssignmentOperator::read_cursor(cursor, source)?;

        cursor.next();
        let modifier = read_optional_expression_modifier(cursor, source)?;
        let rhs = Expression::read_cursor(cursor, source)?;

        cursor.goto_parent();
        Ok(Assignment {
            target: lhs,
            operator,
            modifier,
            expression: rhs,
            span,
        })
    }
}

impl ReadCursor for AssignmentOperator {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        let res = match cursor.kind()? {
            "assign" => Self::Assign,
            "add_assign" => Self::AddAssign,
            "sub_assign" => Self::SubAssign,
            "mul_assign" => Self::MulAssign,
            "pow_assign" => Self::PowAssign,
            "div_assign" => Self::DivAssign,
            "rem_assign" => Self::RemAssign,
            "concat_assign" => Self::ConcatAssign,
            unknown => unreachable!("Unknown assignment operator: {unknown}"),
        };

        Ok(res)
    }
}

impl ReadCursor for Expression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor.node();
        let span = Span::from_node(node);

        cursor_expect!(cursor, "expression");

        cursor.child();

        let kind: ExpressionKind = match cursor.kind()? {
            "else_expression" => ElseExpression::read_cursor(cursor, source)?.into(),
            "match_expression" => {
                ExpressionKind::Match(MatchExpression::read_cursor(cursor, source)?.into())
            }
            "trailing_closure_expression" => read_trailing_closure_call(cursor, source)?.into(),
            "function_call" => FunctionCall::read_cursor(cursor, source)?.into(),
            "postfix_expression" => {
                ExpressionKind::Postfix(PostfixExpression::read_cursor(cursor, source)?.into())
            }
            "argument_modifier_expression" => {
                ExpressionKind::Modified(ModifiedExpression::read_cursor(cursor, source)?.into())
            }
            "constructor_call" => ConstructorCall::read_cursor(cursor, source)?.into(),
            "enum_constructor" => EnumConstructor::read_cursor(cursor, source)?.into(),
            "enum_access" => EnumAccess::read_cursor(cursor, source)?.into(),
            "collection_literal" => CollectionLiteral::read_cursor(cursor, source)?.into(),
            "literal" => Literal::read_cursor(cursor, source)?.into(),
            "ident" => Ident::read_cursor(cursor, source)?.into(),
            "closure" => Closure::read_cursor(cursor, source)?.into(),
            "group" => Group::read_cursor(cursor, source)?.into(),
            _ => ExpressionKind::Infix(InfixExpression::read_cursor(cursor, source)?.into()),
        };

        cursor.goto_parent();

        Ok(Expression { kind, span })
    }
}

fn read_optional_expression_modifier(
    cursor: &mut TreeCursor<'_>,
    source: &str,
) -> Result<Option<DeclModifier>, AstError> {
    if cursor.kind()? == "declaration_modifier" {
        let modifier = DeclModifier::read_cursor(cursor, source)?;
        cursor.next();
        Ok(Some(modifier))
    } else {
        Ok(None)
    }
}
