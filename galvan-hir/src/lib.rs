//! Typed high-level intermediate representation (HIR) for the Galvan language
//! and the typechecker that produces it from the AST.

pub mod builtins;
pub mod error;
pub mod hir;
pub mod index;
pub mod mapping;
pub mod query;
pub mod typecheck;

pub use error::{Diagnostic, DiagnosticSeverity, ErrorCollector, Fix, TranspilerError};
pub use hir::*;
pub use index::{
    render_fn_signature, Definition, DefinitionId, DefinitionKind, Reference, RustItemKind,
    RustLocation, SymbolIndex,
};
pub use typecheck::{typecheck, typecheck_with_interop, Typechecked};
