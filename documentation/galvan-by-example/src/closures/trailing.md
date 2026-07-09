# Trailing Closures

When the last argument of a call is a closure, the argument parentheses can be
dropped entirely. Even non-closure arguments can precede the closure without
parentheses — as in `.fold 2 |acc, quantity| { ... }`:

```galvan
fn main() {
    let order_quantities = [1, 2, 3, 4]

    let total = order_quantities
        .iter()
        .copied()
        .map |quantity| { quantity * 2 }
        .fold 2 |acc, quantity| { acc + quantity }

    assert total == 22
}
```

- `.map |q| { ... }` is `.map(|q| { ... })`.
- `.fold 2 |acc, q| { ... }` is `.fold(2, |acc, q| { ... })`.
- Works with user-defined functions too, including
  [generic helpers](../generics/functions.md).

> [!WARNING]
> **Not implemented yet:** numbered closure parameters (`#0`, `#1`) as an
> alternative to named ones are planned but do not exist yet.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let order_quantities: ::std::vec::Vec<_> = vec![1, 2, 3, 4];
    let total: _ = order_quantities
        .iter()
        .copied()
        .map(|quantity| quantity * 2)
        .fold(2, |acc, quantity| acc + quantity);
    assert_eq!(total, 22,);
}
```

Trailing-closure syntax is purely syntactic — the generated calls are the
ordinary parenthesized forms.

</details>
