# while Loops

`while` iterates as long as its condition is met. `break` and `continue` work as usual:

```galvan
fn main() {
    mut remaining = 3

    while remaining > 0 {
        println("Launching in \(remaining)")
        remaining -= 1
    }
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut remaining: _ = 3;
    while (remaining).gt(&0) {
        println!("{}", &format!("Launching in {}", remaining));
        remaining -= 1;
    };
}
```
</details>
