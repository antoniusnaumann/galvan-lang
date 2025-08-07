# Agent Instructions for Galvan Codebase

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
- Imports: Group stdlib, external crates, then local modules with blank lines between
- Use workspace dependencies in Cargo.toml for shared crates
- Prefer `Box<str>` over `String` for owned string storage when immutable
- Use `#[macro_use] extern crate core` pattern for core functionality
- Trait-based code generation with `Transpile` trait for AST nodes