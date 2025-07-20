use galvan_ast::{
    ArithmeticOperator, CollectionOperator, ComparisonOperator, CustomInfix, EnumAccess,
    Expression, Group, InfixExpression, InfixOperation, InfixOperator, LogicalOperator,
    MemberOperator, Span, TypeIdent, UnwrapOperator,
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for Group {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        todo!()
    }
}

impl ReadCursor for EnumAccess {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "enum_access");
        let span = Span::from_node(node);

        cursor.goto_first_child();

        let target = TypeIdent::read_cursor(cursor, source)?;
        cursor.next();
        cursor_expect!(cursor, "double_colon");
        cursor.next();
        let case = TypeIdent::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(EnumAccess { target, case, span })
    }
}

impl ReadCursor for InfixExpression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let infix = match cursor.kind()? {
            "member_expression" => InfixExpression::Member(
                InfixOperation::<MemberOperator>::read_cursor(cursor, source)?,
            ),
            "logical_expression" => InfixExpression::Logical(
                InfixOperation::<LogicalOperator>::read_cursor(cursor, source)?,
            ),
            "arithmetic_expression" => {
                InfixExpression::Arithmetic(InfixOperation::<ArithmeticOperator>::read_cursor(
                    cursor, source,
                )?)
            }
            "collection_expression" => {
                InfixExpression::Collection(InfixOperation::<CollectionOperator>::read_cursor(
                    cursor, source,
                )?)
            }
            "comparison_expression" => {
                InfixExpression::Comparison(InfixOperation::<ComparisonOperator>::read_cursor(
                    cursor, source,
                )?)
            }
            "unwrap_expression" => InfixExpression::Unwrap(
                InfixOperation::<UnwrapOperator>::read_cursor(cursor, source)?,
            ),
            "custom_infix_expression" => {
                InfixExpression::Custom(InfixOperation::<CustomInfix>::read_cursor(cursor, source)?)
            }
            unknown => unreachable!("Unknown expression kind: {unknown}"),
        };

        Ok(infix)
    }
}

impl<T> ReadCursor for InfixOperation<T>
where
    T: InfixOperator + ReadCursor,
{
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor.child();
        let lhs = Expression::read_cursor(cursor, source)?;
        cursor.next();
        let operator = T::read_cursor(cursor, source)?;
        cursor.next();
        let rhs = Expression::read_cursor(cursor, source)?;
        cursor.goto_parent();

        let operation = InfixOperation { lhs, operator, rhs };
        Ok(operation)
    }
}

impl ReadCursor for MemberOperator {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        let op = match cursor.kind()? {
            "member_call_operator" => Self::Dot,
            "safe_call_operator" => Self::SafeCall,
            unknown => unreachable!("Unknown member operator: {unknown}"),
        };

        Ok(op)
    }
}

impl ReadCursor for LogicalOperator {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        let op = match cursor.kind()? {
            "and" => Self::And,
            "or" => Self::Or,
            "xor" => Self::Xor,
            unknown => unreachable!("Unknown logical operator: {unknown}"),
        };

        Ok(op)
    }
}

impl ReadCursor for ArithmeticOperator {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        let op = match cursor.kind()? {
            "plus" => Self::Add,
            "minus" => Self::Sub,
            "multiply" => Self::Mul,
            "divide" => Self::Div,
            "remainder" => Self::Rem,
            "power" => Self::Exp,
            unknown => unreachable!("Unknown arithmetic operator: {unknown}"),
        };

        Ok(op)
    }
}

impl ReadCursor for CollectionOperator {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        let op = match cursor.kind()? {
            "concat" => Self::Concat,
            "remove" => Self::Remove,
            "contains" => Self::Contains,
            unknown => unreachable!("Unknown collection operator: {unknown}"),
        };

        Ok(op)
    }
}

impl ReadCursor for ComparisonOperator {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        let op = match cursor.kind()? {
            "equal" => Self::Equal,
            "not_equal" => Self::NotEqual,
            "greater" => Self::Greater,
            "greater_equal" => Self::GreaterEqual,
            "less" => Self::Less,
            "less_equal" => Self::LessEqual,
            "identical" => Self::Identical,
            "not_identical" => Self::NotIdentical,
            unknown => unreachable!("Unknown comparison operator: {unknown}"),
        };

        Ok(op)
    }
}

impl ReadCursor for UnwrapOperator {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "unwrap");

        Ok(UnwrapOperator)
    }
}

impl ReadCursor for CustomInfix {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        todo!()
    }
}
