# galvan-lsp — audit follow-up

Working document for the fixes from the 2026-07-03 language-server audit.
Kept up to date as work progresses so anyone can pick up where it stops.
Item numbers refer to the audit report (plan `squishy-humming-flamingo`).

Legend: `[ ]` open · `[x]` done · `[~]` in progress · `[-]` deliberately not done (rationale given)

## Correctness

- [-] **5. Types/members keyed by name only in the symbol index** — duplicate type
  names collapse onto the last declaration. Deliberately deferred: the language
  resolves types by bare name (one namespace per crate), duplicates are already a
  compile error, and a positional key would need scope-aware resolution the
  typechecker doesn't expose. Revisit if/when modules land. Note: there are no separate modules, the only namespaced unit in galvan are crates (and modules as defined by Rust for backward-compatibility)

## Consistent behavior

- [-] **11. Untitled-buffer definitions unreachable in goto/references** — deferred:
  the features already bail earlier for documents without a file path, so the
  `location()` fallback is currently dead code either way. Revisit together with
  proper untitled-buffer support.

## Spec alignment

- [-] **16. `:` as completion trigger fires on single colons** — kept deliberately:
  without it, clients don't auto-trigger after typing `::`, which would break the
  enum-case popup. `context_at` gates the results, so a single `:` yields only
  type-position items.
- [-] **15. `pkg::` / `use` path completion for external crates** — blocked on the
  rustdoc-JSON interop (`galvan-rustdoc`, see repo `todo.md` "Extend Rust interop").
  Tooling can index external symbols only once that data is exposed.
- [-] **17. Grammar gaps** (`@derive` placeholder, `Type.associated_fn()` receiver
  position, `///` doc nodes, `async`, union types) — root causes live in
  `tree-sitter-galvan`, out of LSP scope. See repo `todo.md:79-84`.

## Features

- [x] **20. signatureHelp** — implemented in `features/signature_help.rs`.
  Call sites are found by a forward text scan (string-/comment-/interpolation-
  aware bracket tracking in `call_at`), so help works while the argument list
  is still unclosed and the file does not parse; candidates come from the
  segmented ASTs of every file that parses. Covers free functions (all
  overloads), methods (receiver-filtered through the analysis when available),
  struct/tuple constructors and enum-case constructors. Labelled arguments
  select the parameter by name (constructor arguments may be reordered).
  Tests: `signature_help_*` in `tests/features.rs` (12 cases).
- [x] **23a. semanticTokens** — implemented in `features/semantic_tokens.rs`
  (`textDocument/semanticTokens/full`). Two layers: the tree-sitter parse tree
  supplies comments, string/char/number literals and grammar keyword tokens
  (string interpolations are carved out so the embedded expression highlights
  as code); the symbol index classifies identifiers (function/method/
  struct/enum/enumMember/property/parameter/variable, `declaration` on the
  defining occurrence). Unresolved identifiers fall back to contextual control
  words as keywords and builtins with `defaultLibrary`. Multi-line tokens are
  split per line. Tests: `semantic_tokens_*` in `tests/features.rs`.
- [ ] **23b. code actions** — not started.
- [ ] **23c. formatting** — not started; needs a real formatter, see notes at
  the bottom.

## How to verify

- `cargo test -p galvan-lsp` — e2e tests in `tests/features.rs` drive the same pure
  feature functions the server dispatches to, through the real parser+typechecker.
- `cargo build -p galvan-lsp && python3 galvan-lsp/tests/stdio_smoke.py` — drives the
  real binary over stdio through a full session (initialize, didOpen/diagnostics,
  `::`-completion, hover at end of identifier, documentSymbol, rename, inlayHint,
  didClose clearing diagnostics). Run it after touching `server.rs`; the Rust e2e
  tests do not cover the protocol layer.
- For manual testing: `cargo run -p galvan-lsp` speaks LSP over stdio; point an
  editor at it with `example-projects/*/src/main.galvan`.
