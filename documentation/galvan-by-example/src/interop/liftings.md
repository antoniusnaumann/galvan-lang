# How Rust Types Lift into Galvan

When Galvan imports a crate, it translates the Rust API surface into Galvan
types. Structs, enums, type aliases, free functions, constants, methods, and
trait items all come across. The type translation follows fixed rules:

**Primitives**

| Rust | Galvan |
| --- | --- |
| `bool` | `Bool` |
| `i8` … `i128`, `isize` | `I8` … `I128`, `ISize` |
| `u8` … `u128`, `usize` | `U8` … `U128`, `USize` |
| `f32`, `f64` | `Float`, `Double` |
| `char`, `str`, `String` | `Char`, `String` |

**Collections**

| Rust | Galvan |
| --- | --- |
| `[T]`, `[T; N]`, `Vec<T>` | `[T]` |
| `HashSet<T>` | `{T}` |
| `HashMap<K, V>` | `{K: V}` |
| `IndexMap<K, V>` | `[K: V]` |

**Algebraic types**

| Rust | Galvan |
| --- | --- |
| `Option<T>` | `T?` |
| `Result<T, E>` | `T!E` |
| `anyhow::Result<T>` | `T!` |
| tuples | tuples |
| `fn(A, B) -> R` | `\|A, B\| R` |

**Ownership wrappers**

| Rust | Galvan |
| --- | --- |
| `&T` parameter | `T` (borrow inserted at the call) |
| `&mut T` parameter | `mut T` |
| `Box<T>` | `T` (wrapped/unwrapped at the boundary) |
| `Arc<Mutex<T>>` | `ref T` |

Two properties keep the rules honest:

- **Path-aware:** only *standard* `Vec`, `Option`, `Arc`, … are rewritten. A
  crate's own type named `Option` stays a nominal imported type.
- **Fail-closed:** shapes Galvan cannot represent safely (raw pointers,
  `unsafe` functions, incomplete metadata) are skipped or imported opaquely
  rather than guessed — see [Limits of Interop](limits.md).

> [!NOTE]
> Rust shared references lift to plain Galvan types because pass-by-value
> *is* Galvan's borrow; mutable references lift to `mut` because that is
> Galvan's visible-mutation contract. The lifting table is the ownership
> model of the [Ownership chapter](../ownership/index.md), applied in
> reverse.
