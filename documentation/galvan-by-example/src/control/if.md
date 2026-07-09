# if Expressions

`if` works both as a statement and as an expression. As an expression, the
branches yield the value; an `if` **without** `else` yields an *optional*:

```galvan
fn shipping_status(paid: Bool, reserved: Bool) -> String {
    if paid {
        "ready"
    } else if reserved {
        "waiting for pickup"
    } else {
        "blocked"
    }
}

fn main() {
    let discount = if true { 10 }

    assert shipping_status(true, false) == "ready"
    assert discount == 10
}
```

- `discount` has type `Int?` — `10` if the condition holds, `none` otherwise.
  Optionals get a full [chapter](../errors/optionals.md).
- Comparing an optional against a plain value auto-wraps the plain side, so
  `discount == 10` just works.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn shipping_status(paid: bool, reserved: bool) -> String {
    if paid {
        format!("ready")
    } else {
        if reserved {
            format!("waiting for pickup")
        } else {
            format!("blocked")
        }
    }
}

pub(crate) fn __main__() {
    let discount: Option<_> = if true { Some(10) } else { None };
    assert_eq!(shipping_status(true, false), format!("ready"),);
    assert_eq!(discount, Some(10),);
}
```

The else-less `if` expression is completed with `Some(..)` / `None`, and the
comparison operand is wrapped in `Some` to match.

</details>
