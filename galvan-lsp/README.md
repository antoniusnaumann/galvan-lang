# galvan-lsp

A [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
implementation for the [Galvan](../README.md) programming language.

It is a thin front-end over the Galvan compiler crates — all parsing and name
resolution is reused from the compiler rather than reimplemented:

| Concern                | Crate reused          |
| ---------------------- | --------------------- |
| Reading source text    | `galvan-files`        |
| Parsing (positions)    | `galvan-parse` (tree-sitter) |
| Lowering to AST        | `galvan-into-ast`     |
| AST types              | `galvan-ast`          |
| Name resolution        | `galvan-resolver`     |

## Features

- **Hover** — shows the signature / declaration (and doc comment) of the
  function or type under the cursor.
- **Go to definition** — jumps to the declaration of a top-level function or
  type.
- **Completion** — top-level functions, types, and language keywords.
- **Syntax diagnostics** — surfaced from tree-sitter error nodes.

Limitations (locals, methods, cross-file resolution, semantic diagnostics) are
driven by gaps in the compiler API and are documented in
[`compiler-features.md`](./compiler-features.md). The server degrades gracefully
where a feature is blocked rather than working around the compiler.

## Architecture

```
src/
  main.rs        binary: serves LSP over stdio
  lib.rs         crate root / module docs
  server.rs      tower-lsp Backend: protocol <-> features, owns open documents
  document.rs    Document: text + parse tree + segmented AST per revision
  analysis.rs    symbol-at-offset and name resolution via the compiler crates
  position.rs    LSP position <-> byte offset conversion (LineIndex)
  features/
    hover.rs
    goto_definition.rs
    completion.rs
    diagnostics.rs
```

Each feature is a pure function of a `Document` and request parameters, which
keeps `server.rs` thin and makes the features unit-testable (see
`tests/features.rs`).

## Running

```sh
cargo run -p galvan-lsp
```

The server then speaks LSP over stdin/stdout. Point your editor's LSP client at
this binary for `.galvan` / `.gv` files.
