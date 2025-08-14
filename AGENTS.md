# Agent Instructions for Galvan Codebase

## IMPORTANT
- NEVER edit, delete or uncomment tests to resolve failing tests. Exception: you can modify tests you added during your session.
- AVOID redundant comments: Do not add comments that merely explain what the code does, instead use comments only when further context is needed
- ALWAYS run all workspace tests after finishing an edit session to verify that no regression was introduced
- UPDATE todo.md regularly to reflect current state of TODOs in the codebase - remove completed items and add new ones discovered during development

## Build/Test/Lint Commands
- `cargo check --workspace` - Type check entire workspace
- `cargo build --workspace` - Build entire workspace  
- `cargo test --workspace` - Run all tests
- `cargo test -p <package>` - Run tests for specific package (e.g., `cargo test -p galvan-test`)
- `cargo test <test_name>` - Run specific test by name
- No linting config found - use `cargo clippy` for basic linting

## Code Style & Conventions
- Rust 2021 edition workspace with multiple packages
- Snake_case for module names, functions, variables
- PascalCase for types, traits, enums
- Use `thiserror::Error` for custom error types with `#[from]` for error chaining
- Prefer `itertools::Itertools` for iterator operations (`.join()`, `.collect_vec()`)
- Use derive macros extensively: `#[derive(Debug, PartialEq, Eq)]`
- Error handling: Result types with custom error enums that wrap underlying errors
- Use workspace dependencies in Cargo.toml for shared crates
- Prefer `Box<str>` over `String` for owned string storage when immutable
- Use `#[macro_use] extern crate core` pattern for core functionality
- Trait-based code generation with `Transpile` trait for AST nodes

### Import Organization Standard
Organize imports in exactly 4 groups with blank lines between:
```rust
// Group 1: Standard library
use std::collections::HashMap;
use std::path::Path;

// Group 2: External crates (alphabetical)
use itertools::Itertools;
use thiserror::Error;

// Group 3: Workspace crates (alphabetical)
use galvan_ast::*;
use galvan_files::Source;

// Group 4: Local crate modules (alphabetical)
use crate::context::Context;
use crate::scope::Scope;
```

### Derive Macro Ordering Standard
Order derive macros consistently:
```rust
// Standard order: Clone, Copy, Debug, Default, PartialEq, Eq, Hash, From, Display
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// Custom derives after standard ones
#[derive(Clone, Debug, PartialEq, Eq, AstNode)]
```

### Error Handling Standard
Use `#[from]` with `#[error(transparent)]` for error chaining:
```rust
#[derive(Debug, Error)]
pub enum MyError {
    #[error("Custom error message")]
    Custom,
    
    #[error(transparent)]
    Upstream(#[from] UpstreamError),
}
