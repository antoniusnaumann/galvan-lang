use std::cell::RefCell;

use galvan_ast::{
    AstNode, Block, Body, Closure, ClosureArgument, ConstructorCall, ConstructorCallArg,
    DeclModifier, Expression, FunctionCall, FunctionCallArg, Ident, Return, Span, Statement, Throw,
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
            closure_arguments.push(ClosureArgument::read_cursor(cursor, source)?);
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
        arguments: closure_arguments,
        block,
    };
    // TODO: Insert correct span here that only goes from arguments to closure
    let closure_span = span;
    arguments.push(FunctionCallArg {
        modifier: None,
        expression: Expression {
            kind: closure.into(),
            span: closure_span,
            type_: RefCell::default(),
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
                todo!("TRANSPILER ERROR: Return needs exactly one argument")
            };
            let expression = arguments.into_iter().next().unwrap().expression;
            Statement::Return(Return {
                expression,
                is_explicit: true,
                span,
            })
        }
        "throw" => {
            if arguments.len() > 1 {
                todo!("TRANSPILER ERROR: Throw needs exactly one argument")
            };
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
            type_: RefCell::default(),
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
        let span = Span::from_node(node);

        cursor.child();
        cursor_expect!(cursor, "pipe");
        cursor.next();
        let mut arguments = Vec::new();
        while cursor.kind()? == "closure_argument" {
            arguments.push(ClosureArgument::read_cursor(cursor, source)?);
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
        Ok(Closure { arguments, block })
    }
}

impl ReadCursor for ClosureArgument {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        cursor_expect!(cursor, "closure_argument");

        cursor.child();
        let ident = Ident::read_cursor(cursor, source)?;
        cursor.next();

        let ty = if cursor.kind()? == "colon" {
            cursor.next();
            Some(TypeElement::read_cursor(cursor, source)?)
        } else {
            None
        };

        cursor.goto_parent();
        Ok(ClosureArgument { ident, ty })
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
