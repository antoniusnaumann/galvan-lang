# Optionals and Results

Galvan encodes absence and failure in the type system, with one-character
syntax:

| Type | Meaning | Rust lowering |
| --- | --- | --- |
| `T?` | a `T` or `none` | `Option<T>` |
| `T!E` | a `T` or an error `E` | `Result<T, E>` |
| `T!` | a `T` or a *flexible* error | `galvan::std::FlexResult<T>` (`anyhow::Result`) |

The operators around them follow one rule of thumb: **`?` reads, `!`
propagates.** The safe-call operator `?.` continues only on success, while
postfix `!` unwraps or returns the error early. `else` provides fallbacks,
and `try` branches on success and failure.

Galvan has no `null` and no exceptions — this chapter is the error handling
story, entire.
