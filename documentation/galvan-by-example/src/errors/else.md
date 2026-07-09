# Fallbacks with else

`else` after an optional or result expression unwraps it with a fallback.
The fallback block can compute a value, bind the error, or exit early with
`return`, `throw`, or `panic`:

```galvan
fn load_reward_points(available: Bool) -> Int!Int {
    if available {
        42
    } else {
        throw 21
    }
}

fn main() {
    let points = load_reward_points(true) else { 99 }
    let retry_code = load_reward_points(false) else |error_code| { error_code + 1 }

    assert points == 42
    assert retry_code == 22
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
    let points: i64 = match load_reward_points(true) {
        Ok(__value) => __value,
        Err(it) => 99,
    };
    let retry_code: i64 = match load_reward_points(false) {
        Ok(__value) => __value,
        Err(error_code) => error_code + 1,
    };
    assert_eq!(points, 42,);
    assert_eq!(retry_code, 22,);
}
```

`else` on a result is a `match` on `Ok`/`Err` — like Rust's
`unwrap_or_else`, but with early-exit control flow available in the fallback.

</details>

- On an optional, `else` covers the `none` case.
- On a result, `else` covers the error case — with `|error|` the error value
  is available inside the block.
- `let amount = discount else { return -1 }` mixes unwrapping with early
  return.
