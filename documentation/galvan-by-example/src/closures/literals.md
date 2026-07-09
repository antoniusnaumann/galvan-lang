# Closure Literals

A closure literal is a parameter list between pipes followed by an expression
or block:

```galvan
fn main() {
    let add = |a: Int, b: Int| a + b

    assert add(1, 2) == 3
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let add: impl Fn(i64, i64) -> i64 = |a, b| a + b;
    assert_eq!(add(1, 2), 3,);
}
```

Galvan closures are Rust closures; only the spelled-out binding type is a
problem today (`impl Trait` is not allowed in variable bindings on stable
Rust).

</details>

Parameter types can usually be inferred from the context in which the closure
is used — the annotations above are only needed because nothing else pins the
types down.

> [!WARNING]
> Binding a closure to a variable currently generates an `impl Fn(..)` type
> annotation in `let` position, which stable Rust rejects. Closures passed
> directly as arguments — by far the common case, shown on the next pages —
> work fine.
