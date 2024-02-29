use derive_more::From;
use thiserror::Error;

use crate::Ast;

pub type Result<T> = std::result::Result<T, AstError>;

pub type AstResult = Result<Ast>;

#[derive(Debug, Error, From)]
pub enum AstError {
    #[error("Error when converting parsed code to AST")]
    ConversionError,
    #[error("Error when parsing code")]
    ParseError,
    #[error("Duplicate main function")]
    DuplicateMain,
}
