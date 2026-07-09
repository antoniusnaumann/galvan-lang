# Sharing and Reassigning refs

`ref x = ref y` makes `x` another handle to `y`'s value. Plain assignment to
a `ref` variable writes **through** the reference; assigning `ref other`
**rebinds** the handle to a different target:

```galvan
fn main() {
    ref message = "Hello"
    ref alias = ref message

    message = "Hi"
    println(alias) // Hi — both handles see the write

    ref farewell = "Bye"
    alias = ref farewell // rebind: alias now points at farewell

    println(message) // Hi
    println(alias)   // Bye
}
```

- Write through: `alias = "Bye"` would change `message` too.
- Rebind: `alias = ref farewell` leaves `message` untouched.

Identity of `ref` handles is compared with `===` / `!==` (pointer equality),
while `==` compares the referenced values:

```galvan
let same_target = alias === message
```

> [!WARNING]
> `===` parses and lowers to `Arc::ptr_eq`, but the generated call currently
> passes owned handles where Rust expects borrows and does not compile yet.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut message: std::sync::Arc<std::sync::Mutex<String>> = (&(format!("Hello"))).__to_ref();
    let mut alias: std::sync::Arc<std::sync::Mutex<String>> = ::std::sync::Arc::clone(&message);
    *message.lock().unwrap() = format!("Hi");
    println!("{}", alias.lock().unwrap());
    let mut farewell: std::sync::Arc<std::sync::Mutex<String>> = (&(format!("Bye"))).__to_ref();
    alias = ::std::sync::Arc::clone(&farewell);
    println!("{}", message.lock().unwrap());
    println!("{}", alias.lock().unwrap());
}
```

Non-primitive `ref` values are `Arc<Mutex<T>>`; writes lock, rebinds clone the
`Arc`. `__to_ref()` is a helper from the `galvan` runtime crate that wraps a
value into the shared representation.

</details>
