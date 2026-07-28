# Hello Galvan

Galvan source files use the `.galvan` extension. A Galvan project is a regular
Cargo project: the transpiler runs from `build.rs`, turns every `.galvan` file
under `src/` into Rust, and Cargo compiles the result together with any Rust
crates you depend on.

A minimal project looks like this:

```text
hello-world
├── Cargo.toml
├── build.rs
└── src
    ├── main.galvan
    └── main.rs
```

```toml
# Cargo.toml
[package]
name = "hello-world"
version = "0.1.0"
edition = "2024"

[dependencies]
galvan = "0.0.3"

[build-dependencies]
galvan = { version = "0.0.3", features = ["build"] }
```

```rust
// build.rs
fn main() {
    galvan::setup!();
}
```

```rust
// src/main.rs
galvan::main!();
```

`galvan::setup!()` transpiles your Galvan sources on every build, and
`galvan::main!()` expands to a Rust `fn main` that calls your Galvan entry
point. Everything else is plain Galvan code in `.galvan` files.

This chapter introduces the smallest possible programs: printing text,
formatting values, and the `main` function.
