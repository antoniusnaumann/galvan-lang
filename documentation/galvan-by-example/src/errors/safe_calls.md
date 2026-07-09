# Safe Calls with ?.

The safe-call operator `?.` accesses a member only when the receiver holds a
value; otherwise the whole expression is `none`. Combined with `else` it
reads as "use this, or that":

```galvan
type Dog {
    name: String
}

fn main() {
    let maybe_dog: Dog? = Dog(name: "Hugo")
    let missing: Dog? = none

    let displayed: String = maybe_dog?.name else { "Unknown" }
    let fallback: String = missing?.name else { "Unknown" }

    assert displayed == "Hugo"
    assert fallback == "Unknown"
}
```

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Dog {
    pub(crate) name: String,
}

pub(crate) fn __main__() {
    let maybe_dog: Option<Dog> = Some(Dog {
        name: format!("Hugo"),
    });
    let missing: Option<Dog> = None;
    let displayed: String =
        if let Some(__value) = maybe_dog.as_ref().map(|__elem__| (__elem__.name).clone()) {
            __value
        } else {
            format!("Unknown")
        };
    let fallback: String =
        if let Some(__value) = missing.as_ref().map(|__elem__| (__elem__.name).clone()) {
            __value
        } else {
            format!("Unknown")
        };
    assert_eq!(displayed, format!("Hugo"),);
    assert_eq!(fallback, format!("Unknown"),);
}
```

`?.` lowers to `.as_ref().map(..)` — the receiver is only borrowed, so the
optional remains usable afterwards.

</details>

- `maybe_dog?.name` has type `String?`.
- Safe calls work for methods too: `maybe_score?.double()`.
- Safe calls also skip past the error of a result, yielding an optional.
