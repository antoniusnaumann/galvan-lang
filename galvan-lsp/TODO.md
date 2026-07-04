# galvan-lsp — audit follow-up & improvement roadmap

Working document for the 2026-07-03 language-server audit and the 2026-07-04
improvement roadmap. Kept up to date as work progresses so anyone can pick up
where it stops. Audit item numbers refer to plan `squishy-humming-flamingo`.

**Status: every actionable audit item and every roadmap item is done.** What
remains below is either deliberately deferred with rationale (`[-]`, mostly
blocked on compiler or grammar work) or listed under "Possible next steps".

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

- [x] **A real `galvan-format`** (2026-07-04) — token-level formatter in the new
  `galvan-format` crate: a Wadler-style pretty-printer over the tree-sitter CST
  (spacing, indentation, 100-column reflow of bracketed lists and member
  chains, comment/blank-line preservation, refuses syntax errors). Style
  decisions are documented in `galvan-format/STYLE.md`. The LSP calls it as a
  library and diffs line-by-line into minimal `TextEdit`s; the `galvan-format`
  binary is a stdin/stdout + in-place + `--check` CLI usable from Helix
  (`formatter = { command = "galvan-format" }`).

## Possible next steps

- **Grammar gap: `use` paths with capitalized segments** — `use foo::Bar`
  does not parse (`use_path` only accepts lowercase idents), although the
  README and `example-projects/axum-api` use exactly that form; the axum
  example currently parses as one whole-file error node. Root cause in
  `tree-sitter-galvan`; once fixed, folding's import runs and diagnostics
  automatically improve.
- **More compiler fixes** — the quickfix pipeline (`Diagnostic.code` +
  `Diagnostic.fix` → `quickfix` action) is generic; candidates: unknown
  callees (currently lowered silently for Rust-interop fallthrough),
  `immutable_assignment` (rewrite `let` → `mut` at the declaration),
  argument-label mismatches.
- **signatureHelp for builtin statement functions** (`println`, `assert`, …) —
  they are special-cased in the typechecker and have no `FnDecl` to render;
  needs signature metadata in `galvan_hir::builtins`.
- **semanticTokens/range + delta** — the full-document handler is fast enough
  for now (analysis is memoized per document version); add
  `semantic_tokens_range`/`_full_delta` if large files ever appear.
- **codeLens** — a "run test" lens on `test` blocks once there is a runner
  command to bind it to; reference-count lenses are possible from the index.
- **Call hierarchy** — incoming/outgoing calls are derivable from the
  symbol index's function references.
- **Pull diagnostics (`textDocument/diagnostic`)** — the push model works;
  add the pull handlers when clients demand them.
- **`workspace/didChangeWatchedFiles`** — evict the crate cache when files
  change on disk outside of saves (branch switches, external formatters).
- **analyze() re-parses sources** instead of reusing `CrateFile::segmented` —
  blocked on `Clone` for `SegmentedAsts`/`ToplevelItem` in `galvan-ast`; cost
  is bounded by the per-version memoization, so low priority.
- **galvan-format follow-ups** — sort `use` declarations (needs careful
  comment reattachment), break overlong infix expressions at operators,
  and honor `.editorconfig`/config files for the width options.

## How to verify

- `cargo test -p galvan-lsp` — e2e tests in `tests/features.rs` drive the same pure
  feature functions the server dispatches to, through the real parser+typechecker.
- `cargo test -p galvan-format` — formatter construct-by-construct expectations;
  every case also asserts idempotency, and the example projects must already be
  in canonical style.
- `cargo build -p galvan-lsp && python3 galvan-lsp/tests/stdio_smoke.py` — drives the
  real binary over stdio through a full session (initialize, didOpen/diagnostics,
  `::`-completion, hover at end of identifier, documentSymbol, rename, inlayHint,
  signatureHelp, semanticTokens, codeAction, formatting, documentHighlight,
  foldingRange, typeDefinition, selectionRange, range/on-type formatting, a
  foreign-keyword quickfix round trip, an incremental didChange, didClose
  clearing diagnostics). Run it after touching `server.rs`; the Rust e2e tests
  do not cover the protocol layer.
- For manual testing: `cargo run -p galvan-lsp` speaks LSP over stdio; point an
  editor at it with `example-projects/*/src/main.galvan`.
