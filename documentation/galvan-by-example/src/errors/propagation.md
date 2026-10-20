# Error Propagation with !

Postfix `!` unwraps a result — and if it holds an error, returns it from the
enclosing function immediately. It is Galvan's spelling of Rust's `?`
operator:

```galvan
fn load_reward_points(available: Bool) -> Int!Int {
    if available {
        42
    } else {
        throw 21
    }
}

fn doubled_points(available: Bool) -> Int!Int {
    let points = load_reward_points(available)!
    points * 2
}

fn main() {
    let doubled = doubled_points(true)

    try doubled |points| {
        assert points == 84
    } else {
        panic "expected points"
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

pub(crate) fn doubled_points(available: bool) -> Result<i64, i64> {
    let points: i64 = load_reward_points(available)?;
    Ok(points * 2)
}

pub(crate) fn __main__() {
    let doubled: Result<i64, i64> = doubled_points(true);
    match doubled {
        Ok(points) => {
            assert_eq!(points, 84,);
        }
        Err(it) => {
            panic!("{}", &format!("expected points"));
        }
    };
}
```

Postfix `!` is exactly Rust's `?`.

</details>

> [!NOTE]
> Why `!` and not `?`? In Galvan, `?` always means "continue safely without
> the value" (as in [safe calls](safe_calls.md)), while `!` marks the points
> where a function can bail out. Scanning a function for `!` shows every
> early exit.
