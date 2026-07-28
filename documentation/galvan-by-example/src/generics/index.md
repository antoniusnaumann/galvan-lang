# Generics

Galvan needs no angle-bracket ceremony to introduce type parameters: **any
lowercase type name is generic**. `t`, `u`, `key` — if it starts lowercase,
it is a type parameter; if it starts uppercase, it is a concrete type.

> [!WARNING]
> Generic syntax, generic functions, methods, and `where` clauses are
> implemented, but type inference across generic boundaries and full
> compatibility checking are still incomplete — some generic code emits
> warnings (such as `Type mismatch: expected Container<t>, found Container`)
> or falls back to broad compatibility instead of a precise check.
