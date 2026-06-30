# Compiler features needed by the language server

This file tracks places where `galvan-lsp` is **blocked or limited** by what the
compiler crates currently expose. The language server deliberately does *not*
work around these by reimplementing compiler logic or mutating the other crates;
instead it degrades gracefully and the gap is recorded here so it can be fixed in
the compiler proper.

Each entry notes the impact and where in the LSP code the limitation surfaces.

---

## 1. Identifiers do not carry source spans

`galvan_ast::Ident` and `galvan_ast::TypeIdent` both implement `AstNode::span`
by returning `Span::default()` (there is a `// TODO Save a meaningful span in
this struct` in `galvan-ast/src/item/ident.rs`).

**Impact:** the AST alone cannot tell us which identifier is under the cursor, so
the LSP cannot map a cursor position to an AST identifier node.

**Current handling (not a workaround in the compiler):** the server uses the
tree-sitter parse tree (re-exported by `galvan-parse`) to find the `ident` /
`type_ident` token at a byte offset, then resolves it by *name* against the
`LookupContext`. See `src/analysis.rs::symbol_at`.

**What would help:** store a real `Span` on `Ident`/`TypeIdent`. This would let
the server resolve usages structurally and would be a prerequisite for precise
"find references".

---

## 2. No position-indexed scope information for locals

`galvan_resolver::Scope` models variable scopes, but scopes are constructed
transiently during type checking / transpilation and are not exposed keyed by
source position. There is also no span on local bindings (see #1).

**Impact:** the server cannot resolve **local variables / parameters** for
hover or go-to-definition, and completion cannot offer in-scope locals.

**Current handling:** resolution is limited to top-level **functions** and
**types**. Hovering or go-to-definition on a local simply returns nothing.
Completion offers top-level declarations and keywords only.
See `src/analysis.rs::resolve` and `src/features/completion.rs`.

**What would help:** a query like `scope_at(offset) -> &Scope` (or an exported
map from spans to resolved bindings) produced once during analysis.

---

## 3. Method / receiver resolution needs type inference

`LookupContext::resolve_function` can resolve a method when given the receiver
`TypeIdent` and call labels, but determining the receiver type of an expression
like `dog.name` or `value.method()` requires the typechecker's inferred types,
which are not exposed as a position-queryable API.

**Impact:** go-to-definition and hover only resolve **free functions** (called
without a receiver and without argument labels). Method calls and overloaded
calls distinguished by labels are not resolved.

**Current handling:** `resolve` passes `receiver = None` and `labels = &[]`.
Method calls fall through to "no result".

**What would help:** an API exposing the inferred type of the expression at a
given offset (e.g. from the HIR / typechecker), so the receiver and labels can
be supplied to `resolve_function`.

---

## 4. Cross-*crate* / imported-symbol resolution

Same-*crate* cross-file resolution **is supported**: `src/workspace.rs` loads
every `.galvan` file under the crate's source root (via
`galvan_files::read_sources`) and aggregates them into one `LookupContext` with
`LookupContext::add_from`, exactly as the compiler does for a whole crate. Each
`ToplevelItem` carries its originating `Source`, which the LSP turns into a
cross-file `Location`. This is library reuse, not a workaround.

What is **not** supported is resolution across *crate boundaries*:
`LookupContext` has a commented-out `imports` field and a `// TODO: Nested
contexts for resolving names from imported modules` note in
`galvan-resolver/src/lookup.rs`, so symbols brought in via `use` from another
crate / external dependency cannot be resolved.

**Impact:** go-to-definition and hover do not follow `use` imports into other
crates.

**Current handling:** resolution is scoped to the files of the requesting file's
crate (the nearest ancestor `src` directory). See `src/workspace.rs`.

**What would help:** an import-resolution API on the resolver that maps a `use`
path to the defining item (and its `Source`) across crate boundaries.

---

## 5. Semantic diagnostics — implemented (with a small compiler change)

This is now **supported**. `galvan_hir::typecheck(SegmentedAsts)` already
returns an `ErrorCollector` of span-carrying `Diagnostic`s without running the
transpile-to-Rust pipeline, so the LSP runs it over the whole crate and publishes
the results alongside the syntax diagnostics (see `src/features/diagnostics.rs`
and `Crate::diagnostics` in `src/workspace.rs`).

**Surgical compiler change:** diagnostic spans only carried byte offsets, not the
file they came from, so in a multi-file crate they could not be routed back to a
document. `ErrorCollector` gained a `set_current_file` method (`galvan-hir/src/
error.rs`); the typechecker sets it before lowering each top-level item
(`galvan-hir/src/typecheck/mod.rs`), and the file is stamped onto every span
whose `file` is still empty. This also improves the compiler's own multi-file
error messages.

**Remaining caveats** (not blockers, but worth noting):

- Diagnostics emitted *without* a span (via `ErrorCollector::error`, e.g. some
  `InvalidModifier` / `InvalidSyntax` cases) cannot be placed and are not shown.
  Giving those call sites spans in the compiler would surface them.
- The typechecker is run with an empty `RustInterop`, so code that depends on
  imported Rust items may produce false "unknown type/identifier" diagnostics.
  Wiring real interop in (it is expensive — it shells out to rustdoc) is future
  work.
- Crate-level conflicts that surface as a `LookupError` (e.g. duplicate
  top-level declarations) abort typechecking and are not yet reported.
- Only the requested document's diagnostics are published per refresh; an error
  in another crate file appears when that file is itself refreshed.
