# Galvan Language TODOs

## Architecture

The transpiler is split into a typechecker that lowers the AST into a typed
HIR (`galvan-hir/src/typecheck/`) and mechanical code generation from the HIR
(`galvan-transpiler/src/codegen/`). All type, name, and ownership decisions
belong in the typechecker; codegen only renders nodes and the adjustments
stored in them.

Program entry points are declared as `fn main()`, `fn main(args: [String])`,
or `cmd main(...)`. The latter supplies top-level CLI flags while other
commands remain subcommands.

## Critical - Core Language Features

- **Typechecker improvements** (galvan-hir/src/typecheck/)
  - Add support for type unions
  - Resolve function return ownership (returns are currently always treated
    as owned; functions cannot return borrows)
  - Handle inference for alias types (expr.rs, `field_type`)
  - Replace the wildcard treatment of generic type parameters in
    `types_compatible` with real unification/substitution
  - Resolve `self` receiver calls on Rust standard library methods instead of
    falling back to unknown-signature lowering

- **Missing operator implementations**
  - Remove operator `--` for collections (codegen/expression.rs renders a
    placeholder comment)
  - Custom infix operators (typecheck/expr.rs `lower_infix`)
  - Add unary expression support for logical and bitwise not

- **Parameter modifiers in calls** (galvan-hir/src/typecheck/expr.rs `lower_call_args`)
  - Arguments for `let`-modified parameters are not implemented

## High Priority - Language Completeness

- **Iteration** (galvan-hir/src/typecheck/expr.rs `lower_for`)
  - Tuple iteration

- **`ref` variables**
  - Safe-call (`?.`) on ref variables (typecheck/expr.rs `lower_safe_access`)
  - Fix generated derives for structs with `ref` fields (`Arc<Mutex<T>>`
    does not implement `PartialEq`)
  - Complete atomic `ref` operation coverage beyond primitive locals,
    parameters, fields, assignment, arithmetic assignment, reads, and basic
    mut-argument call-boundaries

- **Tuples**
  - Tuple member access (typecheck/expr.rs `field_type`)

## Medium Priority - Error Handling & Validation

- Validate struct field modifier validity (transpile_item/struct.rs)
- Add proper error handling for invalid member function visibility
  (galvan-transpiler/src/lib.rs `transpile_member_functions`)
- Group extension impl blocks by where-clause constraints instead of taking
  the first function's where clause (galvan-transpiler/src/lib.rs)
- Require an explicit `throw` keyword instead of auto-wrapping error values
  in `Err` (galvan-hir/src/typecheck/coerce.rs)
- Output collected warnings from `exec::transpile_dir`
  (galvan-transpiler/src/exec.rs)

## Low Priority - Language Polish

- **Identifier improvements** (galvan-transpiler/src/transpile_item/ident.rs)
  - Implement fully qualified name lookup / module paths

- **Warning cleanup**
  - Silence or handle unused-parameter warnings in the tree-sitter external
    scanner stub (tree-sitter-galvan/src/scanner.c)
  - Investigate the generic-container type mismatch warning emitted while
    building `galvan-test`
  - Route transpiler `ErrorCollector` diagnostics through a caller-owned sink
    instead of printing Cargo messages from the public transpilation path
  - Normalize existing Rust formatting drift so `cargo fmt --all --check`
    can be used as a clean verification step without unrelated diffs

- **Closure types** (galvan-transpiler/src/transpile_item/type.rs)
  - Let users declare `Fn` instead of `FnMut` closures, e.g. for
    multithreading

- **Tree-sitter grammar completeness** (tree-sitter-galvan/)
  - Add const/async keyword support
  - Replace annotation placeholder with actual implementation
  - Add implicit closure parameter rules
  - Allow type identifiers in member-call receiver position so
    `TypeName.associated_function()` parses as an expression

## Future Enhancements

- Rustdoc toolchain configuration follow-ups:
  - Add manifest-level rustdoc configuration if Galvan gains a project config
    file.
  - Consider an explicit rustup auto-install command for the pinned toolchain,
    gated behind user intent rather than mutating the environment during
    detection.
  - Evaluate auto-detecting compatible installed nightly toolchains now that
    the golden rustdoc fixture guards schema compatibility.
- Extend Rust interop beyond rustdoc-backed free functions:
  - Typecheck namespaced method calls such as `value.crate_name::method()`
  - Resolve external-target function and constant re-exports from rustdoc JSON;
    external type re-exports without target metadata are imported as empty types
  - Support parser/grammar syntax for imported Rust constants with uppercase
    names and qualified constant paths
  - Support qualified external Rust type paths in Galvan type syntax; rustdoc
    metadata preserves module paths, but imported Rust types currently become
    unqualified only through `use`
  - Surface ambiguous unqualified associated Rust function and constant lookups
    as diagnostics once qualified Galvan paths are available end to end
  - Surface ambiguous unqualified Rust `use` imports as diagnostics instead of
    suppressing the lookup silently
  - Replace synthetic generic parameter names on resolved-only dependency type
    placeholders with the real Rust names once the declaring rustdoc item is
    available
  - Key rustdoc field, constructor, and enum variant conversion metadata by
    qualified Rust type path once Galvan has qualified type syntax, so same-named
    imported Rust types from different modules can carry distinct conversion
    metadata instead of suppressing ambiguous unqualified conversion lookups
  - Wire parsed `Ticket.new()` / `Router.new()` syntax into the existing
    typechecker support for imported inherent associated functions
  - Extend safe Rust wrapper lifting beyond the current common cases (`Option<T>`,
    Rust list/map/set collections, `Result<T, E>`, `Arc<Mutex<T>>` /
    `Arc<RwLock<T>>` / `Arc<Atomic*>`, and `Box<T>` / `Rc<T>` interop
    conversions) to cover additional smart pointers and standard wrappers
  - Lift the remaining safe rustdoc type shapes needed for API round-tripping,
    including `dyn Trait`, `impl Trait`, associated type projections, and
    generic associated types
  - Extend imported public Rust data declarations beyond the current named
    struct fields, tuple struct fields, enum variants, and type aliases to
    cover safe modeling for union fields and repr details, full trait
    declarations beyond opaque trait placeholders with associated items,
    explicit Galvan syntax for generic type declarations, and Rust
    lifetime/const generic parameters
  - Infer all Galvan passing modes from lifted Rust signatures beyond owned
    copy/value params, mutable refs, shared borrowed refs, and parameter-side
    owned wrapper conversions, including the remaining receiver/argument cases
    not yet covered by rustdoc import
  - Improve generic substitution and trait-bound handling for external Rust APIs
  - Add qualified type-path syntax to the parser/AST/typechecker and wire it to
    rustdoc's preserved namespace/module paths so same-named Rust types can be
    addressed from Galvan without ambiguity
- Support full Axum-style API declarations in Galvan:
  - Add async functions and `.await`
  - Generate async `main` with the default Tokio runtime
  - Resolve type-associated Rust methods and constants with Galvan member
    syntax, such as `Router.new()` and `StatusCode.CREATED`
  - Support builtin auto traits, `@derive(...)`, `@derive(!Trait)` opt-outs,
    and user-declared `auto trait`s
  - Support shared-state interop from Galvan `ref` fields
- Add "todo" and "panic" as special handling functions
- Implement build entry points and custom tasks (galvan-into-ast/src/items/toplevel.rs)
- Add nested contexts for imported module name resolution (galvan-resolver/src/lookup.rs)
- Improve span tracking throughout AST nodes (most HIR nodes synthesize
  `Span::default()` for derived types)
- Consider a structured Rust code generator (e.g. ruast) instead of string
  formatting (galvan-transpiler/src/lib.rs)

---
*Last updated: 2026-07-09*
*This file should be updated regularly as TODOs are completed or new ones are discovered*
