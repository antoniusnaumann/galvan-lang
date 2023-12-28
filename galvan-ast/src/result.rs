use derive_more::From;
use from_pest::{ConversionError, Void};
use galvan_pest::ParserError;
use thiserror::Error;

use crate::Ast;

pub type Result<T> = std::result::Result<T, AstError>;

pub type AstResult = Result<Ast>;

#[derive(Debug, Error, From)]
pub enum AstError {
    #[error("Error when converting parsed code to AST: {0}")]
    ConversionError(ConversionError<Void>),
    #[error("Error when parsing code: {0}")]
    ParseError(ParserError),
    #[error("Duplicate main function")]
    DuplicateMain,
}
