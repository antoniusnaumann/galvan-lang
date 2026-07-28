# Mutable Parameters

If a function should mutate the caller's value, mark the parameter `mut`.
The caller must acknowledge the mutation at the call site — either with the
`mut` prefix or the postfix `.mut` form:

```galvan
fn make_uppercase(mut name: String) {
    name = name.to_uppercase()
}

fn main() {
    mut name = "milo"

    make_uppercase(mut name)
    make_uppercase(name.mut)

    println(name) // MILO
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn make_uppercase(name: &mut String) {
    {
        *name = name.to_uppercase();
    };
}

pub(crate) fn __main__() {
    let mut name: String = format!("milo");
    make_uppercase(&mut name);
    make_uppercase(&mut name);
    println!("{}", &name);
}
```

`mut` parameters are mutable Rust references. Galvan does not expose
immutable references in user code — the default value mode covers reading, so
`&mut` is the only reference type that surfaces from this feature.

</details>

- Both call spellings are equivalent; postfix `.mut` reads better in chains.
- Only `mut` (or `ref`) bindings can be passed to `mut` parameters.
- Forgetting the call-site annotation is an error — a call that mutates
  its arguments is always visible as such.
