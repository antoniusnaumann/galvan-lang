# Command-Line Interfaces

Galvan builds CLI argument parsing into the language: declare a `cmd` instead
of a `fn` and its parameters become command-line flags, its doc comments
become help text, and subcommands, `--help`, and `--version` come for free.

Under the hood the generated parser uses the battle-tested Rust
[`clap`](https://docs.rs/clap) crate — Galvan writes the derive structs you
would otherwise write by hand.
