# Galvan by Example

[Galvan](https://github.com/antoniusnaumann/galvan-lang) is a high-level
application language with concise syntax, value-oriented defaults, and explicit
escape hatches when shared mutable state is needed. It transpiles to Rust and
uses the Rust ecosystem directly: crates, tooling, and the compiler itself.

Galvan is aimed at application code — CLI tools, services, and other
high-level programs. It is not intended to replace Rust for low-level systems
work; when you need explicit control over memory and lifetimes, write that
part in Rust and call it from Galvan.

*Galvan by Example* is a collection of small examples that illustrate the
language feature by feature, in the spirit of
[Rust by Example](https://doc.rust-lang.org/rust-by-example/). Each chapter
builds on the previous ones, so reading front to back gives a gentle learning
curve — but every page also stands on its own if you are looking something up.

## How to read this book

Most examples end with a collapsible **Generated Rust** section:

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let name: String = format!("Galvan");
    print!("{}", &format!("Welcome to {}!", name));
}
```

</details>

These snippets are produced by the actual Galvan transpiler. They are lightly
trimmed for readability — the surrounding module wrapper, lint attributes, and
re-export plumbing are removed, and the code is formatted with `rustfmt` — but
the items and bodies are exactly what Galvan emits. If you already know Rust,
reading them is the fastest way to build a precise mental model of what each
Galvan construct does. Two artifacts of the generated code are worth knowing up
front:

- A Galvan `fn main` becomes `__main__`, called by a tiny generated `fn main`.
- String literals are rendered as `format!(...)`, because Galvan strings are
  owned Rust `String` values.

Galvan is a work in progress. Everything shown without a warning reflects
implemented, tested behavior. Designed-but-unfinished features are marked like
this:

> [!WARNING]
> **Not implemented yet.** This page describes intended behavior that the
> transpiler does not support yet.

These warnings make the book double as a roadmap: they document the intended
design so that future development has a consistent target.

## Hello World

The obligatory first program:

```galvan
fn main() {
    print("Hello World!")
}
```

Read on — the [first chapter](hello/index.md) starts here.
