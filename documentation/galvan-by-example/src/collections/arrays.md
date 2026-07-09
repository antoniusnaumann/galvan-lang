# Arrays

`[T]` is an ordered, growable collection. Literals infer their element type;
an empty literal takes it from the annotation:

```galvan
fn main() {
    mut daily_orders = [1, 2, 3]

    daily_orders ++= 4
    daily_orders[0] = 10

    assert daily_orders[0] == 10
    assert daily_orders.len() == 4

    let typed: [Int] = []
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut daily_orders: ::std::vec::Vec<_> = vec![1, 2, 3];
    daily_orders.push(4);
    daily_orders[0] = 10;
    assert_eq!(daily_orders[0], 10,);
    assert_eq!(daily_orders.len(), 4,);
    let typed: ::std::vec::Vec<i64> = vec![];
}
```

</details>

- `++=` appends an element (or another array — see
  [Collection Operators](operators.md)).
- Indexing reads with `[i]`; index assignment requires a `mut` binding.
- Rust's `Vec` methods (`len`, `push`, `contains`, …) are available directly.
