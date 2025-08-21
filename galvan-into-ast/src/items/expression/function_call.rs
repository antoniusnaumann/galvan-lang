use galvan_ast::{
    AstNode, Block, Body, Closure, ClosureParameter, ConstructorCall, ConstructorCallArg,
    DeclModifier, EnumAccess, EnumConstructor, EnumConstructorArg, Expression, FunctionCall, FunctionCallArg, Ident, Return, Span, Statement, Throw,
    TypeElement, TypeIdent,
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for FunctionCall {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "function_call");

        cursor.child();
        let identifier = Ident::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "paren_open");

        cursor.next();
        let arguments = read_arguments(cursor, source)?;
        cursor.goto_parent();

        Ok(FunctionCall {
            identifier,
            arguments,
        })
    }
}

pub fn read_trailing_closure_call(
    cursor: &mut TreeCursor<'_>,
    source: &str,
) -> Result<FunctionCall, AstError> {
    let node = cursor_expect!(cursor, "trailing_closure_expression");
    let span = Span::from_node(node);

    cursor.child();
    let identifier = Ident::read_cursor(cursor, source)?;

    cursor.next();
    let mut arguments = read_arguments(cursor, source)?;

    let mut closure_arguments = Vec::new();
    if cursor.kind()? == "pipe" {
        cursor.next();
        while cursor.kind()? != "pipe" {
            closure_arguments.push(ClosureParameter::read_cursor(cursor, source)?);
            cursor.next();
            while cursor.kind()? == "," {
                cursor.next();
            }
        }
        cursor.next();
    }

    let body = Body::read_cursor(cursor, source)?;
    let body_span = body.span;
    let block = Block {
        body,
        span: body_span,
    };

    let closure = Closure {
        parameters: closure_arguments,
        block,
    };
    // TODO: Insert correct span here that only goes from arguments to closure
    let closure_span = span;
    arguments.push(FunctionCallArg {
        modifier: None,
        expression: Expression {
            kind: closure.into(),
            span: closure_span,
        },
    });

    cursor.goto_parent();
    Ok(FunctionCall {
        identifier,
        arguments,
    })
}

pub fn read_free_function_call(
    cursor: &mut TreeCursor<'_>,
    source: &str,
) -> Result<Statement, AstError> {
    let node = cursor_expect!(cursor, "free_function");
    let span = Span::from_node(node);

    cursor.child();
    let identifier = Ident::read_cursor(cursor, source)?;
    cursor.next();
    let arguments = read_arguments(cursor, source)?;
    cursor.goto_parent();

    let call = match identifier.as_str() {
        "return" => {
            // TODO: Allow return without argument
            if arguments.len() > 1 {
                return Err(AstError::ConversionError);
            }
            let expression = arguments.into_iter().next().unwrap().expression;
            Statement::Return(Return {
                expression,
                is_explicit: true,
                span,
            })
        }
        "throw" => {
            if arguments.len() > 1 {
                return Err(AstError::ConversionError);
            }
            let expression = arguments.into_iter().next().unwrap().expression;
            Statement::Throw(Throw { expression, span })
        }
        _ => Statement::Expression(Expression {
            kind: FunctionCall {
                identifier,
                arguments,
            }
            .into(),
            span,
        }),
    };
    Ok(call)
}

fn read_arguments(
    cursor: &mut TreeCursor<'_>,
    source: &str,
) -> Result<Vec<FunctionCallArg>, AstError> {
    let mut args = vec![];

    while cursor.kind()? == "function_call_arg" {
        args.push(FunctionCallArg::read_cursor(cursor, source)?);
        if !cursor.next() {
            break;
        }
        while cursor.kind()? == "," {
            cursor.next();
        }
    }

    Ok(args)
}

impl ReadCursor for FunctionCallArg {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "function_call_arg");

        cursor.child();
        let modifier = if cursor.kind()? == "declaration_modifier" {
            let decl_mod = Some(DeclModifier::read_cursor(cursor, source)?);
            cursor.next();

            decl_mod
        } else {
            None
        };

        let expression = Expression::read_cursor(cursor, source)?;
        cursor.goto_parent();

        let arg = FunctionCallArg {
            modifier,
            expression,
        };
        Ok(arg)
    }
}

impl ReadCursor for Closure {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "closure");
        let _span = Span::from_node(node);

        cursor.child();
        cursor_expect!(cursor, "pipe");
        cursor.next();
        let mut arguments = Vec::new();
        while cursor.kind()? == "closure_argument" {
            arguments.push(ClosureParameter::read_cursor(cursor, source)?);
            cursor.next();
            while cursor.kind()? == "," {
                cursor.next();
            }
        }

        cursor_expect!(cursor, "pipe");
        cursor.next();

        let block = if cursor.kind()? == "expression" {
            let expression = Expression::read_cursor(cursor, source)?;
            let span = expression.span();
            let body = Body {
                statements: vec![expression.into()],
                span,
            };

            Block { body, span }
        } else if cursor.kind()? == "body" {
            let body = Body::read_cursor(cursor, source)?;
            let span = body.span;

            Block { body, span }
        } else {
            unreachable!(
                "Expected 'body' or 'expression' in closure but got: {}",
                cursor.kind().unwrap()
            )
        };

        cursor.goto_parent();
        Ok(Closure {
            parameters: arguments,
            block,
        })
    }
}

impl ReadCursor for ClosureParameter {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "closure_argument");

        cursor.child();
        let ident = Ident::read_cursor(cursor, source)?;
        cursor.next();

        let ty = if cursor.kind()? == "colon" {
            cursor.next();
            TypeElement::read_cursor(cursor, source)?
        } else {
            TypeElement::infer()
        };

        cursor.goto_parent();
        Ok(ClosureParameter { ident, ty })
    }
}

impl ReadCursor for ConstructorCall {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "constructor_call");

        cursor.child();
        let identifier = TypeIdent::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "paren_open");

        let mut arguments = vec![];
        cursor.next();
        while cursor.kind()? != "paren_close" {
            let arg = ConstructorCallArg::read_cursor(cursor, source)?;
            arguments.push(arg);
            cursor.next();
            while cursor.kind()? == "," {
                cursor.next();
            }
        }

        cursor.goto_parent();

        let constructed = ConstructorCall {
            identifier,
            arguments,
        };
        Ok(constructed)
    }
}

impl ReadCursor for ConstructorCallArg {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "constructor_call_arg");

        cursor.child();
        let ident = Ident::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "colon");

        cursor.next();
        let expression = Expression::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(ConstructorCallArg { ident, expression })
    }
}

impl ReadCursor for EnumConstructor {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "enum_constructor");

        cursor.child();
        let enum_access = EnumAccess::read_cursor(cursor, source)?;

        cursor.next();
        cursor_expect!(cursor, "paren_open");

        let mut arguments = vec![];
        cursor.next();
        while cursor.kind()? == "enum_constructor_arg" {
            let arg = EnumConstructorArg::read_cursor(cursor, source)?;
            arguments.push(arg);
            cursor.next();
            while cursor.kind()? == "," {
                cursor.next();
            }
        }

        cursor_expect!(cursor, "paren_close");
        cursor.goto_parent();

        Ok(EnumConstructor {
            enum_access,
            arguments,
        })
    }
}

impl ReadCursor for EnumConstructorArg {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "enum_constructor_arg");

        cursor.child();

        // Simplified parsing - try to read field/value structure
        let mut field_name = None;
        let mut modifier = None;
        
        // Check the first child to determine the structure
        let first_kind = cursor.kind()?;
        
        match first_kind {
            "declaration_modifier" => {
                // Anonymous argument with modifier
                modifier = Some(DeclModifier::read_cursor(cursor, source)?);
                cursor.next();
            }
            _ => {
                // Look for the pattern: if we have ident followed by colon, it's a named field
                if first_kind == "ident" {
                    let current_position = cursor.node();
                    let ident = Ident::read_cursor(cursor, source)?;
                    cursor.next();
                    
                    if cursor.kind()? == "colon" {
                        // Named field
                        field_name = Some(ident);
                        cursor.next();
                    } else {
                        // Need to backtrack - this is actually part of the expression
                        // For now, let's treat it as an error and simplify
                        return Err(AstError::ConversionError);
                    }
                }
            }
        }

        // Read the expression
        let expression = Expression::read_cursor(cursor, source)?;
        cursor.goto_parent();

        Ok(EnumConstructorArg {
            field_name,
            modifier,
            expression,
        })
    }
}
