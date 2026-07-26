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
  type, including declarations in other files of the same crate.
- **Completion** — top-level functions and types from across the crate, plus
  language keywords.
- **Diagnostics** — syntax errors from the parser plus semantic (type) errors
  from the compiler's typechecker, mapped to precise ranges.

Resolution is crate-wide: every `.galvan` file under the crate's `src` root is
indexed (open buffers use their unsaved contents, the rest are read from disk),
mirroring how the compiler treats a crate.

Limitations (locals, methods, cross-*crate* imports, semantic diagnostics) are
driven by gaps in the compiler API and are documented in
[`compiler-features.md`](./compiler-features.md). The server degrades gracefully
where a feature is blocked rather than working around the compiler.

## Architecture

```
src/
  main.rs        binary: serves LSP over stdio
  lib.rs         crate root / module docs
  server.rs      tower-lsp Backend: protocol <-> features, owns open documents
  document.rs    Document: text + parse tree per open buffer (syntactic view)
  workspace.rs   Crate: crate-wide index of all files, for resolution
  analysis.rs    symbol-at-offset and name resolution via the compiler crates
  position.rs    LSP position <-> byte offset conversion (LineIndex)
  features/
    hover.rs
    goto_definition.rs
    completion.rs
    diagnostics.rs
```

Each feature is a pure function of a `Document` / `Crate` and request
parameters, which keeps `server.rs` thin and makes the features unit-testable
(see `tests/features.rs`).

## Running

```sh
cargo run -p galvan-lsp
```

The server then speaks LSP over stdin/stdout. Point your editor's LSP client at
this binary for `.galvan` / `.gv` files.
