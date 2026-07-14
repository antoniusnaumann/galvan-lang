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
    println!("{}", &format!("{}", counter));
}
```

Primitive `ref` values lower to lock-free atomics (`Arc<AtomicI64>` here) with
`SeqCst` ordering; other types use `Arc<Mutex<T>>`. Compound assignments
without a dedicated atomic instruction lower to a `fetch_update`
compare-and-swap loop so the whole read-modify-write stays atomic.

> [!WARNING]
> Some `ref` codegen corners are still rough — for example printing an
> atomic-backed `ref` directly (as the last line above does) currently emits
> Rust that does not compile. Reading into a `let` first, comparisons, and
> arithmetic are solid and covered by the test suite.

</details>

- `ref` values read and write with normal syntax — no explicit locking.
- A `ref` variable can also be passed to a `mut` parameter
  (`bump(counter.mut)`) or by value; only sharing requires `ref` at the call
  site.
