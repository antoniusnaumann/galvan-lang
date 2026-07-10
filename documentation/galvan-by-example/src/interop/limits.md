# Limits of Interop

Galvan deliberately does not bridge everything. Rust surfaces that cannot be
represented safely are excluded, and incomplete metadata fails closed rather
than guessing:

**Never bridged** — write a safe Rust wrapper instead:

- raw pointers and functions whose signatures contain them
- `unsafe` functions and unsafe function pointer types
- non-Rust ABI function pointers
- union fields and `repr` details (unions import as opaque types)

**Not lifted yet** — safe shapes on the roadmap:

- `dyn Trait` and `impl Trait`
- associated type projections (`<T as Trait>::Item`) and generic associated
  types
- lifetime and const generic parameters (ignored today)
- qualified Rust type paths in Galvan type syntax — imported types are
  currently addressed by their unqualified name after `use`

When a function or constant uses an unliftable type, it is skipped. When a
struct or enum would expose one, the type is imported opaquely — it exists
and can be passed around, but its fields are not accessible from Galvan.
Ambiguous items (two crates exporting the same unqualified name, same-named
types from different modules) can be resolved using the fully qualified path
syntax or a fully qualified `use` that imports a specific item, such as
`use axum::http::StatusCode`. Galvan does not pick one over the other when
importing a whole namespace with `use`.

> [!NOTE]
> The authoritative and most current description of these rules is
> `galvan-rustdoc/LIFTINGS.md` in the repository — it is the contract that
> the interop test suite enforces.
