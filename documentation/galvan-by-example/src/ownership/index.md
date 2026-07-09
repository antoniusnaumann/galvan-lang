# Ownership

Galvan replaces Rust's borrow-checker vocabulary with three visible modes:

| Mode | Meaning | Rust lowering |
| --- | --- | --- |
| *(default)* | pass/assign **by value** — callee and caller are independent | borrow or clone, chosen by the transpiler |
| `mut` | callee may **mutate the caller's value** | `&mut T` |
| `ref` | **shared reference semantics** — one value, many handles | `Arc<Mutex<T>>` / `Arc<Atomic*>` |

Two rules make the system predictable:

1. **Reading is free.** Passing a value the default way never requires
   annotations and never invalidates the original.
2. **Writing is visible.** If a call can change your variable, you can see it
   at the call site: `f(mut x)`, `x.mut.method()`, `f(ref x)`.

There are no lifetimes, no `&`, and no move errors in Galvan source. The
transpiler decides between borrowing and cloning; `ref` is the explicit escape
hatch to opt into shared reference semantics when you actually want shared mutable state.
