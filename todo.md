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

- **Match expressions**
  - Add parser, AST, HIR/typechecking, and code generation support for
    enum variant patterns, tuple payload destructuring, named-field payload
    destructuring, ignored fields, wildcard arms, and expression-valued arms

- **Iteration** (galvan-hir/src/typecheck/expr.rs `lower_for`)
  - For loop on dictionaries and ordered dictionaries
  - For loop on optional and result types
  - Tuple iteration

- **`ref` variables**
  - Safe-call (`?.`) on ref variables (typecheck/expr.rs `lower_safe_access`)
  - Fix generated derives for structs with `ref` fields (`Arc<Mutex<T>>`
    does not implement `PartialEq`)

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

- **Closure types** (galvan-transpiler/src/transpile_item/type.rs)
  - Let users declare `Fn` instead of `FnMut` closures, e.g. for
    multithreading

- **Tree-sitter grammar completeness** (tree-sitter-galvan/)
  - Add const/async keyword support
  - Replace annotation placeholder with actual implementation
  - Add implicit closure parameter rules

## Future Enhancements

- Add "todo" and "panic" as special handling functions
- Implement build entry points and custom tasks (galvan-into-ast/src/items/toplevel.rs)
- Add nested contexts for imported module name resolution (galvan-resolver/src/lookup.rs)
- Improve span tracking throughout AST nodes (most HIR nodes synthesize
  `Span::default()` for derived types)
- Consider a structured Rust code generator (e.g. ruast) instead of string
  formatting (galvan-transpiler/src/lib.rs)

---
*Last updated: 2026-06-24*
*This file should be updated regularly as TODOs are completed or new ones are discovered*
