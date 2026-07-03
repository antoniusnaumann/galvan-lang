# galvan-lsp — audit follow-up

Working document for the fixes from the 2026-07-03 language-server audit.
Kept up to date as work progresses so anyone can pick up where it stops.
Item numbers refer to the audit report (plan `squishy-humming-flamingo`).

Legend: `[ ]` open · `[x]` done · `[~]` in progress · `[-]` deliberately not done (rationale given)

## Correctness

- [x] **1. Cursor at end of identifier misses** — fixed: `contains_token()` in
  `galvan-hir/src/index.rs` is end-inclusive for point queries
  (`definition_at`/`reference_at`). Test: `hover_at_end_of_identifier_still_resolves`.
- [x] **4. First match instead of innermost** — fixed together with 1:
  both point queries pick the smallest covering span (`min_by_key(width)`).
- [x] **2. Parse error in current file silently disables features** — fixed:
  `Crate::file_parses()` added; hover/goto fall back to name-based resolution when
  the current file is absent from the analysis, and `member_completion` probes
  whenever the receiver is not found (not only when the whole crate fails).
  Tests: `member_completion_survives_parse_error_when_siblings_parse`,
  `hover_and_goto_fall_back_when_current_file_has_parse_error`.
  Note: `references` still returns empty for a broken current file (no name-based
  fallback exists for references) — acceptable, revisit if it bites.
- [x] **3. Panic race in `refresh`** — fixed: documents are inserted by the
  `did_open`/`did_change` handlers and `refresh` tolerates concurrently closed docs.
- [x] **6. Latent panic in completion probes** — fixed: `insert_placeholder()` with
  char-boundary guard.
- [x] **7. Out-of-range positions clamp onto the next line** — fixed:
  `position.rs::offset` clamps before the trailing `\n`/`\r\n`.
- [-] **5. Types/members keyed by name only in the symbol index** — duplicate type
  names collapse onto the last declaration. Deliberately deferred: the language
  resolves types by bare name (one namespace per crate), duplicates are already a
  compile error, and a positional key would need scope-aware resolution the
  typechecker doesn't expose. Revisit if/when modules land.

## Consistent behavior

- [x] **8. Diagnostics lifecycle** — fixed: `refresh` publishes diagnostics for
  every open document of the crate; `did_close` clears the closed document's
  diagnostics; `did_save` evicts the cached crate (re-reads disk) and refreshes.
- [x] **9. Silent failures** — fixed: `read_sources` errors and typechecker panics
  are logged to stderr (`workspace.rs`).
- [x] **10. `crate_root` fallback can scan unrelated directories** — fixed:
  `crate_root` returns `Option` and never falls back to `"."`; rootless documents
  are analyzed in isolation.
- [-] **11. Untitled-buffer definitions unreachable in goto/references** — deferred:
  the features already bail earlier for documents without a file path, so the
  `location()` fallback is currently dead code either way. Revisit together with
  proper untitled-buffer support.

## Performance

- [x] **12. No caching** — fixed: `Crate::analyze()` is memoized per `Crate`
  (OnceLock, returns `Option<&Analysis>`), and `Backend` caches the `Crate` per
  crate root keyed by the open documents' `(uri, version)` set — so disk reads,
  parsing and typechecking happen once per document version, not once per request.
  Known limitation (documented in `server.rs`): edits made on disk while no open
  document changes are only picked up on the next open/change/save.
  Remaining sub-item: `analyze()` still re-parses sources instead of reusing
  `CrateFile::segmented`, because `SegmentedAsts`/`ToplevelItem` don't implement
  `Clone` (galvan-ast). With memoization this now costs one extra parse per
  document version — low priority; fix by deriving `Clone` in galvan-ast and
  merging the per-file `SegmentedAsts` in `analyze()`.
- [ ] **13. `LineIndex` rebuilt per result location** in references. Fix: build one
  index per distinct source file.

## Spec alignment

- [x] **14. Keyword list vs grammar** — fixed: keyword groups in `completion.rs`
  mirror `tree-sitter-galvan/grammar/keywords.js` (see the comment above the
  constants). Contextual statement starters (`if for while loop try return throw`)
  and word operators (`and or not`) are still offered; `async const main struct
  enum` are not. Built-in statement functions `print println assert panic` are
  offered as FUNCTION items (they never appear in the symbol index). `move` is
  also recognized as a binding keyword in `context_at`/`colon_introduces_type`.
  Test: `completion_keywords_match_the_grammar`.
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

- [ ] **18. Rename + prepareRename** — new `features/rename.rs` on top of
  `SymbolIndex::references` + definition span; skip builtins/synthetic spans.
- [ ] **19. documentSymbol / workspaceSymbol** — new `features/symbols.rs` from
  `index.definitions()`: types with field/variant/method children, free functions.
- [x] **21. Shadowed locals appear twice in completion** — fixed: deduped by name
  in `value_completion`, latest declaration before the cursor wins.
  Test: `completion_dedupes_shadowed_locals`.
- [ ] **20. signatureHelp** — needs call-site argument-index detection; not started.
- [ ] **22. Inlay type hints** for `let` bindings — `Definition::ty()` has the data;
  not started.
- [ ] **23. semanticTokens / code actions / formatting** — future work, largest
  items; not started.

## How to verify

`cargo test -p galvan-lsp` (e2e tests in `tests/features.rs` drive the same pure
feature functions the server dispatches to, through the real parser+typechecker).
For manual testing: `cargo run -p galvan-lsp` speaks LSP over stdio; point an editor
at it with `example-projects/*/src/main.galvan`.
