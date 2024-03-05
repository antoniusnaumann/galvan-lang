use derive_more::From;
use thiserror::Error;

use galvan_parse::{Node, ParseError, TreeCursor};

use crate::Ast;

pub type AstResult = Result<Ast, AstError>;

#[derive(Debug, Error, From)]
pub enum AstError {
    #[error("Error when converting parsed code to AST")]
    ConversionError,
    #[error("Error when parsing code")]
    ParseError(ParseError),
    #[error("Error when parsing item")]
    NodeError,
    #[error("Duplicate main function")]
    DuplicateMain,
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
    fn curr(&self) -> Result<&str, AstError>;
}

impl CursorUtil for TreeCursor<'_> {
    fn curr(&self) -> Result<&str, AstError> {
        Ok(self.node().err()?.kind())
    }
}
