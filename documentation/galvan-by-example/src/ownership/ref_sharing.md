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

- Write through: `alias = "Bye"` would change `message` too.
- Rebind: `alias = ref farewell` leaves `message` untouched.

Identity of `ref` handles is compared with `===` / `!==` (pointer equality),
while `==` compares the referenced values:

```galvan
fn main() {
    ref message = "Hello"
    ref alias = ref message
    ref copy = "Hello"

    assert alias === message
    assert copy !== message
    assert copy == message
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut message: std::sync::Arc<std::sync::Mutex<String>> = (&(format!("Hello"))).__to_ref();
    let mut alias: std::sync::Arc<std::sync::Mutex<String>> = ::std::sync::Arc::clone(&message);
    let mut copy: std::sync::Arc<std::sync::Mutex<String>> = (&(format!("Hello"))).__to_ref();
    assert!(::std::sync::Arc::ptr_eq(&alias, &message));
    assert!(!::std::sync::Arc::ptr_eq(&copy, &message));
    assert!(::galvan::std::__ref_value_eq(&(copy), &(message)),);
}
```

</details>

Value equality acquires distinct mutexes in a stable order, avoiding
same-handle and reversed-comparison deadlocks.
