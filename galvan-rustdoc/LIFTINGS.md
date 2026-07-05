# Rustdoc Liftings

This document describes the Rust shapes that `galvan-rustdoc` currently lifts
into Galvan declarations. It is the contract for rustdoc-backed dependency
interop, not a wishlist.

## Imported Items

`galvan-rustdoc` imports public items from rustdoc JSON and exposes them to the
Galvan typechecker as dependency declarations:

- public structs with named public fields
- public tuple structs
- public enum variants, including tuple and struct variants
- public type aliases with lifted target types
- public traits as opaque dependency types
- public unions as opaque dependency types
- public free functions and constants
- public inherent associated functions, methods, and constants
- public trait methods, trait associated constants, trait-impl methods, and
  trait-impl associated constants
- rustdoc re-exports for local type, function, constant, and glob targets

External type re-exports without target metadata are imported as empty types
when their name looks like a type. External function and constant re-exports
without target metadata are not imported yet.

Associated functions and associated constants can be queried by namespace and
receiver. Public trait methods and trait associated constants use the trait name
as their associated receiver. Unqualified associated item lookup is available
only when the receiver/name pair identifies a single imported Rust item; if
multiple namespaces expose the same unqualified associated item, the lookup is
suppressed until the caller uses a namespace-qualified path.
Inside imported impl and trait associated function signatures, Rust `Self`
types are substituted with the associated receiver type before Galvan sees the
signature.

`use namespace` and `use namespace::item` expose dependency items for
unqualified lookup only when the imported unqualified name is not ambiguous
across the active `use` declarations. Ambiguous unqualified type, function, and
constant imports are suppressed; qualified namespace lookup remains available.

## Primitive Types

Rust primitive and builtin rustdoc type forms lift as follows:

| Rust | Galvan |
| --- | --- |
| `!` | `!` |
| `bool` | `Bool` |
| `i8`, `i16`, `i32`, `i64`, `i128`, `isize` | `I8`, `I16`, `I32`, `I64`, `I128`, `ISize` |
| `u8`, `u16`, `u32`, `u64`, `u128`, `usize` | `U8`, `U16`, `U32`, `U64`, `U128`, `USize` |
| `f32`, `f64` | `Float`, `Double` |
| `char` | `Char` |
| `str` | `String` |

Resolved Rust `String` also lifts to Galvan `String` and is treated as a
builtin type rather than an imported dependency type.

Unknown rustdoc primitive names lift to `__UnknownRustPrimitive` so the imported
surface remains visible instead of silently disappearing.

## Generic And Path Types

Rust generic type parameters lift to Galvan generic type parameters with the
same name. Public Rust type declarations also preserve their type generic
parameter list, including opaque imported types whose fields are not exposed.
Rust lifetime parameters and const generic parameters are ignored because
Galvan does not currently have corresponding API-surface syntax.

Resolved Rust paths lift by their item name. If the resolved type has generic
arguments, those arguments are lifted recursively and preserved as Galvan
parametric type arguments. If the resolved type is not one of the known wrapper
types below, `galvan-rustdoc` records the type as an imported dependency type.
When only a referenced path is available, the imported dependency placeholder
keeps the generic arity with names from generic use-site arguments where
possible and stable synthetic names otherwise. Full Rust type declarations use
the generic parameter names from rustdoc.
When rustdoc provides the resolved module path, that path is preserved in the
Rust metadata so same-named Rust types from different modules remain distinct.
Self-crate rustdoc prefixes such as `crate::` and `$crate::` are normalized to
the imported crate name before Galvan stores Rust paths.

Known generic wrappers are lifted only when rustdoc includes the required type
arguments. Incomplete wrapper metadata is treated as unliftable rather than
filled with inferred Galvan types.

Qualified external type paths are preserved in the rustdoc metadata, but Galvan
type syntax does not yet expose qualified type paths all the way through the
parser and typechecker. Imported dependency types are therefore currently used
through their unqualified Galvan names after `use`.

## References And Passing Modes

Rust shared references lift to the referenced Galvan type. Parameter-side shared
references record a call conversion that borrows the Galvan argument when
calling Rust.

Rust mutable references lift to the referenced Galvan type with a `mut`
declaration modifier.

Borrowed return values are marked as borrowed metadata. The HIR and transpiler
use that metadata to insert the necessary owned conversion when Galvan code
needs an owned value.

## Collections

Rust collection types lift to Galvan collection types:

| Rust | Galvan |
| --- | --- |
| `[T]`, `[T; N]`, `Vec<T>`, `VecDeque<T>`, `LinkedList<T>` | `[T]` |
| `HashSet<T>`, `BTreeSet<T>`, `IndexSet<T>` | `{T}` |
| `HashMap<K, V>` | `{K: V}` |
| `BTreeMap<K, V>`, `IndexMap<K, V>` | `[K: V]` |

Fixed array lengths are not preserved in the lifted Galvan type.

## Algebraic Types

`Option<T>` lifts to `T?`.

`Result<T, E>` lifts to `T!E`. `galvan::std::FlexResult<T>` and
`anyhow::Result<T>` lift to `T!`. If rustdoc does not provide an error type for
another `Result<T>` shape, `galvan-rustdoc` uses `__UnknownRustError`.

Rust tuples lift to Galvan tuples with recursively lifted element types.

Safe Rust-ABI function pointers and bare function types lift to Galvan closure
types `|A, B| R`. Unsafe function pointer types and non-Rust ABI function
pointer types are not lifted.

## Shared State Wrappers

Rust shared synchronization wrappers lift to Galvan `ref` fields and parameters
only when the Rust type carries shared ownership:

| Rust | Galvan |
| --- | --- |
| `Arc<Mutex<T>>`, `Arc<RwLock<T>>` | `ref T` |
| `Arc<AtomicBool>` | `ref Bool` |
| `Arc<AtomicI8>`, `Arc<AtomicI16>`, `Arc<AtomicI32>`, `Arc<AtomicI64>`, `Arc<AtomicIsize>` | `ref I8`, `ref I16`, `ref I32`, `ref I64`, `ref ISize` |
| `Arc<AtomicU8>`, `Arc<AtomicU16>`, `Arc<AtomicU32>`, `Arc<AtomicU64>`, `Arc<AtomicUsize>` | `ref U8`, `ref U16`, `ref U32`, `ref U64`, `ref USize` |

When a shared-state wrapper is consumed, the `Arc`, lock, or atomic wrapper type
is not recorded as part of the Galvan API surface. Bare `Mutex<T>` and
`RwLock<T>` are not lifted to `ref`; they are skipped because Galvan `ref`
represents shared state, and a lock without `Arc` does not provide shared
ownership across the boundary. A lock is recognized as shared state only when it
is the immediate payload of `Arc`, so shapes such as `Option<Mutex<T>>` or
`Vec<RwLock<T>>` are unliftable rather than rewritten to `ref`. Naked `Atomic*`
types remain nominal Rust
dependency types because Galvan only treats atomic primitives as shared `ref`
storage when they appear behind `Arc`. Other `Arc<T>` shapes remain `Arc<T>` in
the lifted Galvan type and are recorded as dependency types. They are not
treated as Galvan `ref` unless the inner type is one of the recognized shared
state wrappers above.

Known wrapper lifting is path-aware when rustdoc provides a path. Standard
library wrappers are lifted from `std`, `core`, or `alloc` paths; `IndexMap` and
`IndexSet` are lifted from the `indexmap` crate; `anyhow::Result<T>` and
`galvan::std::FlexResult<T>` lift to `T!`. Same-named dependency types such as a
crate-local `Option<T>`, `Vec<T>`, `Result<T, E>`, `Arc<T>`, or `Mutex<T>`
remain nominal imported Rust types instead of being rewritten to Galvan wrapper
syntax.

## Owned Wrapper Conversions

`Box<T>` and `Rc<T>` are lifted away at the Galvan boundary for common owned
interop cases:

- function parameters of type `Box<T>` lift as `T` and call Rust with
  `Box::new(argument)`
- function parameters of type `Rc<T>` lift as `T` and call Rust with
  `Rc::new(argument)`
- function returns of type `Box<T>` lift as `T` and dereference the Rust return
  value
- function returns of type `Rc<T>` lift as `T` and clone through the Rust return
  value
- struct fields, tuple struct fields, and enum variant fields using `Box<T>` or
  `Rc<T>` lift as `T` and carry the same constructor, field, and match
  conversions

As with the shared-state wrappers above, these conversions are path-aware when
rustdoc provides a path. Only standard-library `Box` and `Rc` shapes are lifted
away; dependency types with the same names remain nominal imported Rust types.
Owned wrapper conversions require rustdoc metadata for the wrapped type.
Incomplete `Box` or `Rc` conversion metadata is treated as unliftable.
When multiple imported Rust types share the same unqualified Galvan type name,
field, constructor, and enum variant wrapper conversions are suppressed for
that name until Galvan can key those conversions by qualified Rust type paths.

## Explicit Exclusions

`galvan-rustdoc` does not bridge raw pointers, unsafe functions, unsafe function
pointer types, non-Rust ABI function pointer types, or other Rust-only surfaces
that Galvan cannot represent safely. Functions whose signatures contain raw
pointers, currently unliftable type shapes, or incomplete type metadata are
skipped. Constants with unliftable types are skipped. Data declarations whose
public surface contains raw pointers or unliftable type shapes are kept opaque
instead of exposing those fields or variants. Rust unions are imported as opaque
types; union fields and representation details are not exposed in Galvan. Data
declarations are also kept
opaque when rustdoc metadata is incomplete enough that fields or variants would
otherwise be silently dropped, or when rustdoc exposes non-public fields that
Galvan cannot represent as part of a constructible public data declaration. Impl
items are skipped when their receiver type cannot be lifted into a Galvan
associated receiver. If an API requires raw pointers or unsafe contracts, write
that boundary in Rust and expose a safe wrapper to Galvan.

Unknown rustdoc type forms are treated as unliftable. Functions and constants
using them are skipped; data declarations containing them are kept opaque.

The following safe Rust shapes are also not lifted yet:

- `dyn Trait`
- `impl Trait`
- associated type projections such as `<T as Trait>::Item`
- generic associated types
- union fields and `repr` details
- lifetime and const generic parameters
- external function and constant re-exports that do not have local rustdoc
  target metadata
