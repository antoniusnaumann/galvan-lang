# Ordered Dictionaries

`[K: V]` is a dictionary that remembers ordering. It supports the same
indexing, insertion, and iteration as `{K: V}`:

```galvan
fn main() {
    mut daily_menu = [
        "breakfast": 4,
        "lunch": 5,
    ]

    daily_menu["dinner"] = 14

    assert daily_menu["dinner"] == 14

    for daily_menu |meal, servings| {
        assert servings > 3
    }
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut daily_menu: ::std::collections::BTreeMap<String, _> =
        ::std::collections::BTreeMap::from([(format!("breakfast"), 4), (format!("lunch"), 5)]);
    daily_menu.insert(format!("dinner"), 14);
    assert_eq!(daily_menu[&format!("dinner")], 14,);
    for (meal, servings) in &daily_menu {
        assert!((servings).gt(&3));
    }
}
```

</details>

> [!WARNING]
> **Backing type mismatch.** Ordered dictionaries are designed to preserve
> *insertion* order backed by `IndexMap`, but the transpiler currently emits
> `BTreeMap`, which sorts by *key* instead. The API works; the iteration
> order differs from the design until this is fixed.
