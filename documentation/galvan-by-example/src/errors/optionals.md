# Optionals

`T?` holds a `T` or `none`. Plain values auto-wrap wherever an optional is
expected — assignments, arguments, comparisons — so `Some(..)` never appears
in Galvan code:

```galvan
fn count_or_default(count: Int?) -> Int {
    count else { 0 }
}

fn main() {
    let selected: Int? = 5
    let missing: Int? = none

    assert count_or_default(21) == 21
    assert count_or_default(missing) == 0

    for selected {
        assert it == 5
    }
}
```

- `none` is the absent value.
- `count else { 0 }` unwraps with a fallback — see
  [Fallbacks with else](else.md).
- A `for` loop over an optional runs its body only when a value is present.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn count_or_default(count: Option<i64>) -> i64 {
    if let Some(__value) = count {
        __value
    } else {
        0
    }
}

pub(crate) fn __main__() {
    let selected: Option<i64> = Some(5);
    let missing: Option<i64> = None;
    assert_eq!(count_or_default(Some(21)), 21,);
    assert_eq!(count_or_default(missing), 0,);
    for it in selected {
        assert_eq!(*it, 5,);
    }
}
```

Auto-wrapping inserts the `Some(..)` calls; `Option`'s `IntoIterator` gives
the zero-or-one-iteration loop for free.

</details>
