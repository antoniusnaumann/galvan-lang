use thiserror::Error;

use galvan_ast::Span;
use galvan_parse::{Node, ParseError, TreeCursor};

use crate::Ast;

pub type AstResult = Result<Ast, AstError>;

#[derive(Debug, Error)]
pub enum AstError {
    #[error("Error when converting parsed code to AST")]
    ConversionError,
    #[error("Error when parsing item")]
    NodeError,
    #[error("Duplicate main function")]
    DuplicateMain,
    #[error("Invalid character literal at {0:?}")]
    InvalidCharacterLiteral(Span),
    #[error(transparent)]
    Parse(#[from] ParseError),
}

pub trait TreeSitterError: Sized {
    fn err(self) -> Result<Self, AstError>;
}

impl TreeSitterError for Node<'_> {
    fn err(self) -> Result<Self, AstError> {
        if self.is_error() {
            Err(AstError::NodeError)
        } else {
            Ok(self)
        }
    }
}

pub trait CursorUtil {
    fn kind(&self) -> Result<&str, AstError>;
    fn curr(&self) -> Result<Node<'_>, AstError>;
    /// Goes to the next sibling, skipping comments if possible
    fn next(&mut self) -> bool;
    /// Goes to the first child, skipping comments if possible
    fn child(&mut self) -> bool;
}

impl CursorUtil for TreeCursor<'_> {
    fn kind(&self) -> Result<&str, AstError> {
        Ok(self.curr()?.kind())
    }

    fn curr(&self) -> Result<Node<'_>, AstError> {
        Ok(self.node().err()?)
    }

    fn next(&mut self) -> bool {
        let mut res = self.goto_next_sibling();

        while let Ok("comment") = self.kind() {
            if !res {
                break;
            }
            res = self.goto_next_sibling();
        }

        res
    }

    fn child(&mut self) -> bool {
        let mut res = self.goto_first_child();

        while let Ok("comment") = self.kind() {
            if !res {
                break;
            }
            res = self.goto_next_sibling();
        }

        res
    }
}
