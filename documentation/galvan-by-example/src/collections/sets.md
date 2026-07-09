# Sets

`{T}` is an unordered collection of unique values. Membership is tested with
the `in` operator:

```galvan
fn main() {
    mut pantry = {
        "oranges",
        "apples",
    }

    pantry ++= "kiwis"

    assert "kiwis" in pantry
    assert "coffee" in pantry == false
}
```

- `++=` inserts an element; inserting an existing value is a no-op.
- Two sets merge with `++` (union) — see
  [Collection Operators](operators.md).

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut pantry: ::std::collections::HashSet<String> =
        ::std::collections::HashSet::from([format!("oranges"), format!("apples")]);
    pantry.insert(format!("kiwis"));
    assert!((pantry).contains(&(format!("kiwis"))));
    assert_eq!((pantry).contains(&(format!("coffee"))), false,);
}
```

`in` lowers to `.contains(..)` with the borrow inserted automatically.

</details>
