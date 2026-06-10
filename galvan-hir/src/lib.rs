//! Typed high-level intermediate representation (HIR) for the Galvan language
//! and the typechecker that produces it from the AST.

pub mod builtins;
pub mod error;
pub mod hir;
pub mod mapping;

pub use error::{Diagnostic, DiagnosticSeverity, ErrorCollector, TranspilerError};
pub use hir::*;
