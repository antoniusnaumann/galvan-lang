# Dictionaries

`{K: V}` maps keys to values. Indexing reads an entry, index assignment
inserts or updates one, and `for` iterates over key–value pairs:

```galvan
fn main() {
    mut pantry = {
        "oranges": 4,
        "apples": 5,
    }

    pantry["kiwis"] = 21
    pantry["oranges"] = 16

    assert pantry["kiwis"] == 21

    for pantry |item, count| {
        assert count > 3
    }
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut pantry: ::std::collections::HashMap<String, _> =
        ::std::collections::HashMap::from([(format!("oranges"), 4), (format!("apples"), 5)]);
    pantry.insert(format!("kiwis"), 21);
    pantry.insert(format!("oranges"), 16);
    assert_eq!(pantry[&format!("kiwis")], 21,);
    for (item, count) in &pantry {
        assert!((count).gt(&3));
    }
}
```

Index assignment becomes `insert`; reads use Rust's indexing (which panics on
a missing key, matching Galvan's semantics for `[]`).

</details>

Iteration order of a dictionary is unspecified — reach for an
[ordered dictionary](ordered_dictionaries.md) when order matters.
