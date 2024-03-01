use derive_more::From;
use thiserror::Error;

use galvan_parse::ParseError;

use crate::Ast;

pub type AstResult = Result<Ast, AstError>;

#[derive(Debug, Error, From)]
pub enum AstError {
    #[error("Error when converting parsed code to AST")]
    ConversionError,
    #[error("Error when parsing code")]
    ParseError(ParseError),
    #[error("Duplicate main function")]
    DuplicateMain,
}
