# Reference Variables

`ref` declares a variable with **reference semantics**: every handle to it
sees the same value, across function calls and even threads. Passing a `ref`
into a `ref` parameter requires a call-site annotation, mirroring `mut`:

```galvan
fn increment(ref counter: Int) {
    counter += 1
}

fn main() {
    ref counter = 0

    counter += 1
    increment(ref counter)
    increment(counter.ref)

    println("\(counter)") // 3
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn increment(counter: std::sync::Arc<std::sync::Mutex<i64>>) {
    {
        *counter.lock().unwrap() += 1;
    };
}

pub(crate) fn __main__() {
    let mut counter: std::sync::Arc<std::sync::Mutex<i64>> = (&(0)).__to_ref();
    *counter.lock().unwrap() += 1;
    increment(::std::sync::Arc::clone(&counter));
    increment(::std::sync::Arc::clone(&counter));
    println!("{}", &format!("{}", *counter.lock().unwrap()));
}
```

All `ref` values use `Arc<Mutex<T>>`. Reads take a short-lived lock and
mutations operate through a mutex guard, so compound assignments remain one
atomic read-modify-write operation with respect to other users of that mutex.

</details>

- `ref` values read and write with normal syntax — no explicit locking.
- A `ref` variable can also be passed to a `mut` parameter
  (`bump(counter.mut)`) or by value; only sharing requires `ref` at the call
  site.
