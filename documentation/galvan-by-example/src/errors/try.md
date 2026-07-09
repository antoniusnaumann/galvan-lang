# try Expressions

`try` branches on an optional or result: the main block runs with the
unwrapped value, the `else` block handles `none` or the error. The value is
available as `it` or as an explicit `|binding|`; both blocks can yield a
value, making `try` an expression:

```galvan
fn load_reward_points(available: Bool) -> Int!Int {
    if available {
        42
    } else {
        throw 21
    }
}

fn main() {
    let maybe_seats: Int? = if true { 6 }

    try maybe_seats |seats| {
        assert seats == 6
    }

    let booked = try maybe_seats { it + 1 } else { 0 }
    assert booked == 7

    try load_reward_points(false) |points| {
        panic "expected loading to fail"
    } else |error_code| {
        assert error_code == 21
    }
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn load_reward_points(available: bool) -> Result<i64, i64> {
    if available {
        Ok(42)
    } else {
        return Err(21);
    }
}

pub(crate) fn __main__() {
    let maybe_seats: Option<i64> = if true { Some(6) } else { None };
    r#try(maybe_seats, |seats| {
        assert_eq!(*seats, 6,);
    });
    let booked: i64 = match maybe_seats {
        Some((it)) => it + 1,
        None => 0,
    };
    assert_eq!(booked, 7,);
    match load_reward_points(false) {
        Ok(points) => {
            panic!("{}", &format!("expected loading to fail"));
        }
        Err(error_code) => {
            assert_eq!(error_code, 21,);
        }
    };
}
```

`try`/`else` lowers to a `match`; the else-less statement form calls the
small `r#try` helper from the `galvan` runtime crate, which runs the closure
only when a value is present.

</details>

- The `else` branch is optional when `try` is used as a statement.
- On results, `else |error|` binds the error value; with implicit `it`, the
  main block's `it` is the success value and the `else` block's `it` is the
  error.
- `try` borrows — the optional or result stays usable after the expression.
