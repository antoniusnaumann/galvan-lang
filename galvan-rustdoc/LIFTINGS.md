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
- public free functions and constants
- public inherent associated functions, methods, and constants
- public trait-impl methods
- rustdoc re-exports for local type, function, constant, and glob targets

External type re-exports without target metadata are imported as empty types
when their name looks like a type. External function and constant re-exports
without target metadata are not imported yet.

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
When rustdoc provides the resolved module path, that path is preserved in the
Rust metadata so same-named Rust types from different modules remain distinct.

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

Rust function pointers and bare function types lift to Galvan closure types
`|A, B| R`.

## Shared State Wrappers

Rust shared synchronization wrappers lift to Galvan `ref` fields and parameters:

| Rust | Galvan |
| --- | --- |
| `Arc<Mutex<T>>`, `Arc<RwLock<T>>` | `ref T` |
| `Arc<AtomicBool>` | `ref Bool` |
| `Arc<AtomicI8>`, `Arc<AtomicI16>`, `Arc<AtomicI32>`, `Arc<AtomicI64>`, `Arc<AtomicIsize>` | `ref I8`, `ref I16`, `ref I32`, `ref I64`, `ref ISize` |
| `Arc<AtomicU8>`, `Arc<AtomicU16>`, `Arc<AtomicU32>`, `Arc<AtomicU64>`, `Arc<AtomicUsize>` | `ref U8`, `ref U16`, `ref U32`, `ref U64`, `ref USize` |

When `Arc` is consumed as a shared-state wrapper, the `Arc`, lock, or atomic
wrapper type is not recorded as part of the Galvan API surface. Naked
`Mutex<T>`, `RwLock<T>`, and `Atomic*` types remain nominal Rust dependency
types because they do not represent shared ownership by themselves. Other
`Arc<T>` shapes remain `Arc<T>` in the lifted Galvan type and are recorded as
dependency types. They are not treated as Galvan `ref` unless the inner type is
one of the recognized shared state wrappers above.

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

## Explicit Exclusions

`galvan-rustdoc` does not bridge raw pointers, unsafe functions, or other unsafe
Rust-only surfaces into Galvan. Functions whose signatures contain raw pointers
or currently unliftable type shapes are skipped. Constants with unliftable types
are skipped. Data declarations whose public surface contains raw pointers or
unliftable type shapes are kept opaque instead of exposing those fields or
variants. Data declarations are also kept opaque when rustdoc metadata is
incomplete enough that fields or variants would otherwise be silently dropped.
If an API requires raw pointers or unsafe contracts, write that boundary in Rust
and expose a safe wrapper to Galvan.

The following safe Rust shapes are also not lifted yet:

- `dyn Trait`
- `impl Trait`
- associated type projections such as `<T as Trait>::Item`
- generic associated types
- unions and `repr` details
- lifetime and const generic parameters
- external function and constant re-exports that do not have local rustdoc
  target metadata
