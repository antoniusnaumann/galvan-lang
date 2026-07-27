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
    let add = |a, b| a + b;
    assert_eq!(add(1, 2), 3,);
}
```

Galvan closures are Rust closures. The local binding leaves its type inferred,
because Rust does not permit spelling a closure's anonymous type in a `let`
annotation.

</details>

Parameter types can usually be inferred from the context in which the closure
is used — the annotations above are only needed because nothing else pins the
types down.
