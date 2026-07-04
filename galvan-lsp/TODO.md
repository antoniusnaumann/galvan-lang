# galvan-lsp — audit follow-up & improvement roadmap

Working document for the 2026-07-03 language-server audit and the 2026-07-04
improvement roadmap. Kept up to date as work progresses so anyone can pick up
where it stops. Audit item numbers refer to plan `squishy-humming-flamingo`.

**Status: every actionable audit item and every roadmap item is done.** What
remains below is either deliberately deferred with rationale (`[-]`, mostly
blocked on compiler or grammar work) or listed under "Possible next steps".

Legend: `[ ]` open · `[x]` done · `[~]` in progress · `[-]` deliberately not done (rationale given)

## Improvement roadmap (2026-07-04)

- [x] **Structured diagnostics** — `galvan_hir::Diagnostic` carries a stable
  `code` (one per `TranspilerError` variant, see `TranspilerError::code`) and
  an optional machine-applicable `fix` (span + replacement). The Levenshtein
  did-you-mean helpers attach the matched candidate as a fix. The LSP forwards
  both through `Diagnostic.code`/`Diagnostic.data`.
- [x] **Foreign-keyword recognition** — `features/foreign_syntax.rs` maps
  keywords from other languages to Galvan (`func`/`def` → `fn`, `switch` →
  `match`, `class`/`struct` → `type`, `var` → `mut`, `null` → `none`,
  `import` → `use`, …; `&&`/`||` are excluded because the grammar already
  accepts them). Three detection layers: tree-sitter error regions (skipping
  comments/strings), unresolved callees of calls that *parse*
  (`switch color { }` is a trailing-closure call the typechecker lowers
  silently), and unknown-identifier/type diagnostics. Tests:
  `foreign_keyword_*` in `tests/features.rs`.
- [x] **Quickfix code actions** — every context diagnostic carrying fix data
  becomes a preferred `quickfix` action (`Replace \`func\` with \`fn\``,
  did-you-mean typo fixes). Decoding lives in `features/diagnostics.rs`
  (`decode_fix`); the actions in `features/code_actions.rs`. Tests:
  `quickfix_*`.
- [x] **Completion familiarity aliases** — keyword items match foreign
  spellings via `filter_text` (typing `switch` offers `match`), scoped to the
  keyword sets they belong to and sorted after real keywords. Tests:
  `completion_aliases_*`.
- [x] **documentHighlight / foldingRange / selectionRange / typeDefinition /
  rangeFormatting / onTypeFormatting** — one pure module each; folding covers
  multi-line bracket pairs, comment runs and import runs; on-type formatting
  triggers on `}`; partial formats reuse the document formatter so they can
  never disagree with it. Tests: `document_highlight_*`, `folding_ranges_*`,
  `selection_range_*`, `type_definition_*`, `range_formatting_*`,
  `on_type_formatting_*`.
- [x] **Code quality** — the thirteen semantic handlers share
  `Backend::request_context`; text sync is INCREMENTAL (ranged edits applied
  in order, whole-document changes still accepted); the smoke test resolves
  the binary relative to itself (`GALVAN_LSP_BIN` overrides).

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
- [x] **23b. code actions** — implemented in `features/code_actions.rs`.
  "Add type annotation" (`refactor.rewrite`) writes the inferred type of an
  unannotated local into the source; the inference is shared with inlay hints
  (`inlay_hints::unannotated_locals`), and each inlay hint now also carries
  the same insertion as its `text_edits`. Diagnostics carry no structured fix
  data yet, so there are no quickfixes — new actions should follow the
  pattern in that module. Tests: `code_action_*` /
  `inlay_hints_carry_the_annotation_as_text_edit` in `tests/features.rs`.
- [x] **23c. formatting** — implemented in `features/formatting.rs` as a
  deliberately *scoped* whitespace formatter: it normalizes leading
  indentation (one unit per open bracket, dedent on leading closers, one
  extra unit for `.`/`?.` member-chain continuations) and strips trailing
  whitespace, and it never reflows tokens across lines, so it cannot change
  program meaning. Multi-line string content is protected via the parse
  tree; files that do not parse are refused. Conformance is pinned by
  `formatting_leaves_the_example_projects_unchanged` (zero edits on the
  hand-formatted example projects) and an idempotence test. Extending it
  into a full token-reflowing formatter (line-length limits, spacing rules)
  would be a compiler-side project (`galvan-format`), not an LSP patch.

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
- **A real `galvan-format`** — token-level formatting (spacing, line-length
  reflow) belongs in a compiler-side crate the LSP would call into; the LSP's
  whitespace formatter is deliberately limited to indentation and trailing
  whitespace.

## How to verify

- `cargo test -p galvan-lsp` — e2e tests in `tests/features.rs` drive the same pure
  feature functions the server dispatches to, through the real parser+typechecker.
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
