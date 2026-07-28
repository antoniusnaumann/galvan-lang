# Iterators

Rust's iterator adapters are available directly on Galvan collections. Galvan
adds `.vec()` as a shorthand for `.collect::<Vec<_>>()`, and its trailing
closure syntax makes chains read like pipeline stages:

```galvan
fn main() {
    let order_quantities = [1, 2, 3, 4, 5, 6, 7, 8]

    let packed = order_quantities
        .iter()
        .copied()
        .map |quantity| { quantity * 2 }
        .filter |quantity| { quantity % 4 == 0 }
        .vec()

    assert packed == [4, 8, 12, 16]

    for order_quantities.iter().zip(packed) |original, doubled| {
        assert original * 2 == doubled * 1
    }
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let order_quantities: ::std::vec::Vec<_> = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let packed: _ = order_quantities
        .iter()
        .copied()
        .map(|quantity| quantity * 2)
        .filter(|quantity| (quantity % 4).eq(&0))
        .vec();
    assert_eq!(packed, vec![4, 8, 12, 16],);
    for (original, doubled) in order_quantities.iter().zip(&packed) {
        assert_eq!(original * 2, doubled * 1,);
    }
}
```

The chain maps one-to-one onto Rust iterator adapters. `.vec()` comes from a
small extension trait in the `galvan` runtime crate.

</details>

- `.map |x| { ... }` is a [trailing closure](../closures/trailing.md) —
  argument parentheses are optional when the last argument is a closure.
- Any iterator expression can be the subject of a `for` loop; `zip` pairs
  unpack into multiple loop bindings.
