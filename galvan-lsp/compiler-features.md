# Compiler features needed by the language server

This file tracks how `galvan-lsp` uses the compiler crates and where it is
still **blocked or limited** by what they expose. The language server does not
reimplement compiler logic; features degrade gracefully and gaps are recorded
here so they can be fixed in the compiler proper.

## Resolved

The following former shortcomings are now implemented **in the compiler** and
consumed by the LSP:

### 1. Identifiers carry source spans — resolved

`galvan_ast::Ident` and `galvan_ast::TypeIdent` store the span of their token
(`Ident::spanned`, populated by `galvan-into-ast`). Spans are not part of an
identifier's identity (equality/hashing use only the name), so identifiers
remain usable as lookup keys.

### 2. Position-indexed scope information — resolved

Typechecking now produces a `galvan_hir::SymbolIndex` as a by-product
(`typecheck` returns a `Typechecked { module, index, errors }`). While
lowering, the checker records:

- **definitions**: top-level functions and types, struct fields, enum
  variants, parameters and local bindings (each with the span of its
  defining identifier and the enclosing scope for locals),
- **references**: every identifier use the checker resolves — variable reads,
  function and method calls, type annotations, constructor calls, field
  accesses, enum accesses and match patterns,
- **scopes**: the byte range in which each local scope's bindings are
  visible.

Queries: `symbol_at(file, offset)`, `references(id)`,
`visible_locals(file, offset)`, `definitions()`. The LSP builds hover,
go-to-definition, find-references and scope-aware completion directly on
these (see `src/analysis.rs` and `src/features/`).

### 3. Inferred types are position-queryable — resolved

Every `HirExpression` stores its inferred type and span;
`galvan_hir::query::expression_at(&module, file, offset)` returns the
innermost expression at a position. The LSP uses this for member completion
(receiver type of the expression before a `.`) and for hover over arbitrary
expressions. Method calls resolve through the symbol index (recorded during
type inference), so the LSP never re-runs receiver inference itself.

### 5. Semantic diagnostics — resolved

`galvan_hir::typecheck(SegmentedAsts)` returns span-carrying `Diagnostic`s
without running the transpile-to-Rust pipeline. Additionally:

- Diagnostic spans carry the file they belong to (`ErrorCollector::
  set_current_file`), so multi-file crates route errors to the right
  document.
- All typechecker diagnostics now carry spans (the former span-less
  `ErrorCollector::error` call sites were converted).
- Duplicate top-level declarations no longer abort typechecking: the first
  declaration wins, the conflict is reported as a `DuplicateDeclaration`
  diagnostic at the second declaration's identifier, and the rest of the
  crate still typechecks (`LookupContext::add_from` returns the conflicts
  instead of `Err`).

## Remaining limitations

### 4. Cross-*crate* / imported-symbol resolution

Same-crate cross-file resolution is supported (the whole crate is loaded and
typechecked as a unit). Resolution across *crate boundaries* is not:
`LookupContext` has no nested contexts for imported modules, so symbols
brought in via `use` from another crate cannot be resolved to their
declaration.

**What would help:** an import-resolution API on the resolver that maps a
`use` path to the defining item (and its `Source`) across crate boundaries.

### 6. Rust interop in the language server

The LSP typechecks with an empty `RustInterop` (building the real one shells
out to rustdoc and is too slow to run per keystroke). Code that uses imported
Rust items may therefore produce false "unknown type/identifier" diagnostics,
and Rust symbols are absent from completion.

**What would help:** a cacheable, offline-buildable interop index (build once
per dependency version, load from disk).

### 7. Extension/UFCS methods in member completion

Member completion offers fields and methods whose declared receiver type
matches the receiver expression. Functions that are callable in method
position via the first-argument rule, and builtin functions registered
without a receiver type (e.g. collection helpers), are not offered after a
dot because the `Definition` does not carry enough signature information to
match them.

**What would help:** recording the first-parameter type on
`DefinitionKind::Function` even when it is not a plain `self` receiver.
