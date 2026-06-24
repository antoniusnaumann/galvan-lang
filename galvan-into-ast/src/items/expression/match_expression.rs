use galvan_ast::{
    Block, Body, Expression, Ident, MatchArm, MatchBindingPattern, MatchEnumPattern,
    MatchExpression, MatchNamedPatternArg, MatchPattern, MatchPatternArg, MatchWildcardPattern,
    Span, TypeIdent,
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for MatchExpression {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "match_expression");

        cursor.child();
        cursor_expect!(cursor, "match_keyword");

        cursor.next();
        let scrutinee = Box::new(Expression::read_cursor(cursor, source)?);

        cursor.next();
        cursor_expect!(cursor, "brace_open");

        cursor.next();
        let mut arms = Vec::new();
        while cursor.kind()? == "match_arm" {
            arms.push(MatchArm::read_cursor(cursor, source)?);
            cursor.next();
        }

        cursor_expect!(cursor, "brace_close");
        cursor.goto_parent();

        Ok(Self { scrutinee, arms })
    }
}

impl ReadCursor for MatchArm {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "match_arm");

        cursor.child();
        let pattern = MatchPattern::read_cursor(cursor, source)?;

        cursor.next();
        let body = Body::read_cursor(cursor, source)?;
        let span = body.span;
        let body = Block { body, span };

        cursor.goto_parent();

        Ok(Self { pattern, body })
    }
}

impl ReadCursor for MatchPattern {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "match_pattern");

        cursor.child();
        let pattern = match cursor.kind()? {
            "wildcard_match_pattern" => {
                MatchPattern::Wildcard(MatchWildcardPattern::read_cursor(cursor, source)?)
            }
            "enum_match_pattern" => {
                MatchPattern::EnumVariant(MatchEnumPattern::read_cursor(cursor, source)?)
            }
            unknown => unreachable!("Unknown match pattern: {unknown}"),
        };
        cursor.goto_parent();

        Ok(pattern)
    }
}

impl ReadCursor for MatchWildcardPattern {
    fn read_cursor(cursor: &mut TreeCursor<'_>, _source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "wildcard_match_pattern");
        let span = Span::from_node(node);

        Ok(Self { span })
    }
}

impl ReadCursor for MatchEnumPattern {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "enum_match_pattern");
        let span = Span::from_node(node);

        cursor.child();
        let case = TypeIdent::read_cursor(cursor, source)?;

        let mut arguments = Vec::new();
        if cursor.next() && cursor.kind()? == "match_pattern_args" {
            cursor.child();
            cursor_expect!(cursor, "paren_open");

            cursor.next();
            while cursor.kind()? == "match_pattern_arg" {
                arguments.push(MatchPatternArg::read_cursor(cursor, source)?);
                cursor.next();
                while cursor.kind()? == "," {
                    cursor.next();
                }
            }

            cursor_expect!(cursor, "paren_close");
            cursor.goto_parent();
        }

        cursor.goto_parent();

        Ok(Self {
            case,
            arguments,
            span,
        })
    }
}

impl ReadCursor for MatchPatternArg {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "match_pattern_arg");

        cursor.child();
        let argument = match cursor.kind()? {
            "named_match_pattern_arg" => {
                MatchPatternArg::Named(MatchNamedPatternArg::read_cursor(cursor, source)?)
            }
            "binding_match_pattern" => {
                MatchPatternArg::Binding(MatchBindingPattern::read_cursor(cursor, source)?)
            }
            unknown => unreachable!("Unknown match pattern argument: {unknown}"),
        };
        cursor.goto_parent();

        Ok(argument)
    }
}

impl ReadCursor for MatchNamedPatternArg {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "named_match_pattern_arg");
        let span = Span::from_node(node);

        cursor.child();
        let field = Ident::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "colon");

        cursor.next();
        let binding = MatchBindingPattern::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(Self {
            field,
            binding,
            span,
        })
    }
}

impl ReadCursor for MatchBindingPattern {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "binding_match_pattern");

        cursor.child();
        let pattern = match cursor.kind()? {
            "ident" => MatchBindingPattern::Ident(Ident::read_cursor(cursor, source)?),
            "wildcard_match_pattern" => {
                MatchBindingPattern::Wildcard(MatchWildcardPattern::read_cursor(cursor, source)?)
            }
            unknown => unreachable!("Unknown match binding pattern: {unknown}"),
        };
        cursor.goto_parent();

        Ok(pattern)
    }
}
