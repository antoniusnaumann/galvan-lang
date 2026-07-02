//! Typed high-level intermediate representation (HIR) for the Galvan language
//! and the typechecker that produces it from the AST.

pub mod builtins;
pub mod error;
pub mod hir;
pub mod index;
pub mod mapping;
pub mod query;
pub mod typecheck;

pub use error::{Diagnostic, DiagnosticSeverity, ErrorCollector, TranspilerError};
pub use hir::*;
pub use index::{Definition, DefinitionId, DefinitionKind, Reference, SymbolIndex};
pub use typecheck::{typecheck, Typechecked};
