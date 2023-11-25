use derive_more::From;
use thiserror::Error;
use from_pest::{ConversionError, Void};
use galvan_pest::ParserError;

use crate::Ast;

pub type AstResult = Result<Ast, AstError>;

#[derive(Debug, Error, From)]
pub enum AstError {
    #[error("Error when converting parsed code to AST: {0}")]
    ConversionError(ConversionError<Void>),
    #[error("Error when parsing code: {0}")]
    ParseError(ParserError),
}