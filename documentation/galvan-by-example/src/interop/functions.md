# Calling Crate Functions

Items from dependency crates are available immediately with
namespace-qualified syntax — no import required. Here `serde_json` is only
listed in `Cargo.toml`:

```galvan
fn main() {
    let scores = [32, 48, 64]
    let payload = serde_json::to_string(scores) else |error| {
        "encoding failed"
    }
    println "scores as json: \(payload)"
}
```

Because `serde_json::to_string` returns a Rust `Result`, it arrives in Galvan
as a [result type](../errors/results.md) — all the error-handling operators
(`else`, `try`, postfix `!`) apply to foreign functions exactly as to local
ones.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let scores: ::std::vec::Vec<_> = vec![32, 48, 64];
    let payload: String = match ::serde_json::to_string(&scores) {
        Ok(__value) => __value,
        Err(error) => {
            format!("encoding failed")
        }
    };
    println!("{}", &format!("scores as json: {}", payload));
}
```

The call goes straight to the crate — Galvan inserts the borrow (`&scores`)
that the Rust signature asks for.

</details>
