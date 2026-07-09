# Results and throw

`T!E` holds a success value of type `T` or an error of type `E`. Success
values auto-wrap just like optionals; errors are raised with `throw`, which
returns early:

```galvan
fn checked_divide(a: Float, b: Float) -> Float!String {
    if b == 0.0 {
        throw "Division by zero"
    }

    a / b
}

fn main() {
    let quotient = checked_divide(10.0, 4.0)

    try quotient |value| {
        assert value == 2.5
    } else |error| {
        panic "expected division to succeed: \(error)"
    }
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn checked_divide(a: f32, b: f32) -> Result<f32, String> {
    if (b).eq(&0.0) {
        return Err(format!("Division by zero"));
    };
    Ok(a / b)
}

pub(crate) fn __main__() {
    let quotient: Result<f32, String> = checked_divide(10.0, 4.0);
    match quotient.to_owned() {
        Ok(value) => {
            assert_eq!(value, 2.5,);
        }
        Err(error) => {
            panic!("{}", &format!("expected division to succeed: {}", error));
        }
    };
}
```

`throw` is `return Err(..)`; the function's final expression is wrapped in
`Ok(..)` automatically.

</details>

- The error type follows the `!`: `Float!String`, `Int!IoError`, …
- Omitting it (`Float!`) selects the *flexible* error type, and a bare `-> !`
  is shorthand for `Void!` — a fallible function with no success payload.
- `throw` accepts any value of the error type.

> [!WARNING]
> **Partially implemented.** Typed errors (`T!E`) are solid. Flexible-error
> results (`T!`, backed by `anyhow`) declare and propagate fine — including
> from [Rust crates](../interop/liftings.md) — but `throw`ing a value into
> one currently generates Rust that misses the `anyhow` conversion and does
> not compile. Bare `-> !` functions additionally miss their implicit `Ok`
> for the success path.
