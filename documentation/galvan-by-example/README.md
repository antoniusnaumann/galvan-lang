# Galvan by Example

An example-driven introduction to the Galvan language, in the spirit of
[Rust by Example](https://doc.rust-lang.org/rust-by-example/). It doubles as
a consistency check for language development: implemented behavior is shown
with real transpiler output, and designed-but-unfinished features carry
explicit `[!WARNING]` alerts.

## Building

The book uses [mdBook](https://rust-lang.github.io/mdBook/):

```sh
cargo install mdbook   # once
mdbook serve           # live-reloading local preview
mdbook build           # writes static HTML to book/
```

## Conventions for contributors

- Every page introduces one feature with a small, self-contained example.
- Examples that transpile end with a collapsible `<details>` block titled
  **Generated Rust** containing the *actual* transpiler output for the
  example, lightly trimmed: the `galvan_module` wrapper, lint attributes, and
  `mod`/`pub use` plumbing are removed, and the result is formatted with
  `rustfmt`. Do not hand-write or "improve" these snippets — regenerate them
  by running the example through `galvan_transpiler::transpile` when the
  transpiler changes.
- Features that do not transpile yet are marked with a `> [!WARNING]` alert
  stating what is missing. When you implement one of these features, update
  the page: remove or narrow the warning and add real generated output.
- Galvan code blocks use the `galvan` language tag; `theme/galvan-highlight.js`
  registers highlighting for it.
