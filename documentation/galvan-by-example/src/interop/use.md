# Imports with use

`use` makes dependency items available *unqualified*. Importing the crate
brings in all of its public items (like Rust's `use crate::*`); importing a
path brings in just that item:

```galvan
use serde_json

fn main() {
    let scores = [32, 48, 64]
    let payload = to_string(scores) else { "encoding failed" }
    println "scores as json: \(payload)"
}
```

<!-- galvan-book: rustdoc-dependent -->

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let scores: ::std::vec::Vec<_> = vec![32, 48, 64];
    let payload: String = match ::serde_json::to_string(&scores) {
        Ok(__value) => __value,
        Err(it) => {
            format!("encoding failed")
        }
    };
    println!("{}", &format!("scores as json: {}", payload));
}
```

</details>

Namespace-qualified access (`serde_json::to_string(..)`) always works, as long as the respective crate is listed as dependency in the `Cargo.toml`.

```galvan
fn main() {
    let scores = [32, 48, 64]
    let payload = serde_json::to_string(scores) else { "encoding failed" }
    println "scores as json: \(payload)"
}
```

<!-- galvan-book: rustdoc-dependent -->

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let scores: ::std::vec::Vec<_> = vec![32, 48, 64];
    let payload: String = match ::serde_json::to_string(&scores) {
        Ok(__value) => __value,
        Err(it) => {
            format!("encoding failed")
        }
    };
    println!("{}", &format!("scores as json: {}", payload));
}
```

Galvan resolves the import at transpile time and emits the fully qualified
path either way.

</details>


```galvan
use serde_json::to_string
```

This path-only form is a declaration fragment rather than a complete program.
It changes name resolution but emits no standalone Rust item; calls made
through the imported name still lower to `::serde_json::to_string`.

If two imported crates export the same name, the unqualified import is
suppressed and the qualified syntax remains — ambiguity is never resolved
silently in favor of one crate.
