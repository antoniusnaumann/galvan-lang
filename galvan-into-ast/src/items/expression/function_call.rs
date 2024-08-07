use galvan_ast::{
    Block, Body, Closure, ClosureArgument, ConstructorCall, ConstructorCallArg, DeclModifier, Expression, FunctionCall, FunctionCallArg, Ident, Span, TypeElement, TypeIdent
};
use galvan_parse::TreeCursor;

use crate::{cursor_expect, result::CursorUtil, AstError, ReadCursor, SpanExt};

impl ReadCursor for FunctionCall {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "function_call");
        let span = Span::from_node(node);

        cursor.goto_first_child();
        let identifier = Ident::read_cursor(cursor, source)?;

        cursor.goto_next_sibling();
        cursor_expect!(cursor, "paren_open");
        
        cursor.goto_next_sibling();
        let arguments = read_arguments(cursor, source)?;
        cursor.goto_parent();

        Ok(FunctionCall { identifier, arguments, span})
    }
}

pub fn read_trailing_closure_call(
    cursor: &mut TreeCursor<'_>,
    source: &str,
) -> Result<FunctionCall, AstError> {
    todo!()
}

pub fn read_free_function_call(
    cursor: &mut TreeCursor<'_>,
    source: &str,
) -> Result<FunctionCall, AstError> {
    let node = cursor_expect!(cursor, "free_function");
    let span = Span::from_node(node);

    cursor.goto_first_child();
    let identifier = Ident::read_cursor(cursor, source)?;
    cursor.goto_next_sibling();
    let arguments = read_arguments(cursor, source)?;
    cursor.goto_parent();

    let func = FunctionCall {
        identifier,
        arguments,
        span,
    };
    Ok(func)
}

fn read_arguments(
    cursor: &mut TreeCursor<'_>,
    source: &str,
) -> Result<Vec<FunctionCallArg>, AstError> {
    let mut args = vec![];

    while cursor.kind()? == "function_call_arg" {
        args.push(FunctionCallArg::read_cursor(cursor, source)?);
        cursor.goto_next_sibling();
    }
    
    // println!("args: {}", args.len());
    Ok(args)
}

impl ReadCursor for FunctionCallArg {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "function_call_arg");
        let span = Span::from_node(node);

        cursor.goto_first_child();
        let modifier = if cursor.kind()? == "declaration_modifier" { 
            let decl_mod = Some(DeclModifier::read_cursor(cursor, source)?);
            cursor.goto_next_sibling();
                
            decl_mod
        } else { None };

        let expression = Expression::read_cursor(cursor, source)?;
        cursor.goto_parent();

        let arg = FunctionCallArg {
            modifier,
            expression,
            span,
        };
        Ok(arg)
    }
}

impl ReadCursor for Closure {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "closure");
        let span = Span::from_node(node);

        cursor.goto_first_child();
        cursor_expect!(cursor, "pipe");
        cursor.goto_next_sibling();
        let mut arguments = Vec::new();
        while cursor.kind()? == "closure_argument" {
            arguments.push(ClosureArgument::read_cursor(cursor, source)?);
            cursor.goto_next_sibling();
        }

        cursor_expect!(cursor, "pipe");
        cursor.goto_next_sibling();

        let body = Body::read_cursor(cursor, source)?;
        let block_span = body.span.clone();
        let block = Block { body, span: block_span };

        cursor.goto_parent();
        Ok(Closure { arguments, block, span })
    }
}

impl ReadCursor for ClosureArgument {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "closure_argument");
        let span = Span::from_node(node);

        cursor.goto_first_child();
        let ident = Ident::read_cursor(cursor, source)?;
        cursor.goto_next_sibling();
        
        let ty = if cursor.kind()? == "colon" {
            cursor.goto_next_sibling();
            Some(TypeElement::read_cursor(cursor, source)?)
        } else { None };

        cursor.goto_parent();
        Ok(ClosureArgument { ident, ty, span })
    }
}

impl ReadCursor for ConstructorCall {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "constructor_call");
        let span = Span::from_node(node);

        cursor.goto_first_child();
        let identifier = TypeIdent::read_cursor(cursor, source)?;

        cursor.goto_next_sibling();
        cursor_expect!(cursor, "paren_open");

        let mut arguments = vec![];
        cursor.goto_next_sibling();
        while cursor.kind()? != "paren_close" {
            let arg = ConstructorCallArg::read_cursor(cursor, source)?;
            arguments.push(arg);
            cursor.goto_next_sibling();
        }

        cursor.goto_parent();

        let constructed = ConstructorCall {
            identifier,
            arguments,
            span,
        };
        Ok(constructed)
    }
}

impl ReadCursor for ConstructorCallArg {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError> {
        let node = cursor_expect!(cursor, "constructor_call_arg");
        let span = Span::from_node(node);

        cursor.goto_first_child();
        let ident = Ident::read_cursor(cursor, source)?;

        cursor.goto_next_sibling();
        cursor_expect!(cursor, "colon");

        cursor.goto_next_sibling();
        let expression = Expression::read_cursor(cursor, source)?;

        cursor.goto_parent();

        Ok(ConstructorCallArg {
            ident,
            expression,
            span,
        })
    }
}
