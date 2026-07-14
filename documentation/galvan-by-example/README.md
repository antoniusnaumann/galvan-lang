# Galvan by Example

An example-driven introduction to the Galvan language, in the spirit of
[Rust by Example](https://doc.rust-lang.org/rust-by-example/). It doubles as
a consistency check for language development: implemented behavior is shown
with real transpiler output, and designed-but-unfinished features carry
explicit `[!WARNING]` alerts.

## Building

The book uses [mdBook](https://rust-lang.github.io/mdBook/):

```sh
cargo install mdbook --version 0.4.51 --locked
cargo install mdbook-alerts --version 0.8.0 --locked
mdbook serve           # live-reloading local preview
mdbook build           # writes static HTML to book/
```

## Conventions for contributors

- Every page introduces one feature with a small, self-contained example.
- Every transpiling example is followed **directly** by a collapsible
  `<details>` block titled **Generated Rust** — placed immediately after the
  code block it documents, never collected at the end of the page — containing
  the *actual* transpiler output for the example, lightly trimmed: the `galvan_module` wrapper, lint attributes, and
  `mod`/`pub use` plumbing are removed, and the result is formatted with
  `rustfmt`. Do not hand-write or "improve" these snippets — regenerate them
  with `cargo run -p galvan-transpiler --bin galvan-book -- --write` when the
  transpiler changes. Run the same command without `--write` to check for
  drift.
- Examples that require dependency rustdoc metadata carry a
  `<!-- galvan-book: rustdoc-dependent -->` marker before their generated
  output. Verify those in a fixture crate that declares the dependency; the
  local checker deliberately skips them instead of accepting passthrough
  codegen as authoritative output.
- Features that do not transpile yet are marked with a `> [!WARNING]` alert
  stating what is missing. When you implement one of these features, update
  the page: remove or narrow the warning and add real generated output.
- Galvan code blocks use the `galvan` language tag; `theme/galvan-highlight.js`
  registers highlighting for it.
