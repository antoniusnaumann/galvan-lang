# Pass by Value

Arguments are passed by value by default: the function gets its own copy, and
the caller's variable is untouched.

```galvan
fn shouted(name: String) -> String {
    name.to_uppercase()
}

fn main() {
    let name = "milo"
    let loud = shouted(name)

    println(name) // milo — still valid
    println(loud) // MILO
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn shouted(name: &str) -> String {
    name.to_uppercase()
}

pub(crate) fn __main__() {
    let name: String = format!("milo");
    let loud: String = shouted(&name);
    println!("{}", &name);
    println!("{}", &loud);
}
```

The `String` parameter lowers to `&str`: value semantics at the surface,
borrow underneath. This is the central trick of Galvan's ownership model —
copy-on-write-style defaults with zero annotations.

</details>

By-value does **not** mean a deep copy at runtime. The transpiler passes a
borrow whenever the callee only reads the value, and clones only where
ownership is genuinely needed (for example when storing the argument in a
struct or calling a Rust function that demands an owned parameter).
