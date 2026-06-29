//! Language server for the Galvan programming language.
//!
//! The crate is a thin LSP front-end over the Galvan compiler libraries: it
//! reuses [`galvan_parse`], [`galvan_into_ast`] and [`galvan_resolver`] for all
//! parsing and name resolution rather than re-implementing them. Each LSP
//! feature lives in its own module under [`features`] and is a pure function of
//! a [`document::Document`], which makes them independently testable.
//!
//! Where the compiler does not yet expose information the server needs, the gap
//! is documented in `compiler-features.md` and the feature degrades gracefully
//! instead of working around it.

pub mod analysis;
pub mod document;
pub mod features;
pub mod position;
pub mod server;

pub use server::Backend;
