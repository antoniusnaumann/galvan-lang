# Defining Functions

Functions are declared with `fn`. The final expression of the body is the
return value — no `return` keyword needed. `return` exists for early exits:

```galvan
fn add(a: Int, b: Int) -> Int {
    a + b
}

fn fib(n: Int) -> Int {
    if n <= 1 {
        return n
    }

    fib(n - 1) + fib(n - 2)
}

fn main() {
    println("\(add(2, 3))")
    println("\(fib(10))")
}
```

- Parameters are typed `name: Type`; the return type follows `->`.
- A function without `->` returns nothing (Rust's `()`).
- `pub fn` exports a function from the crate; plain `fn` is crate-local.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn add(a: i64, b: i64) -> i64 {
    a + b
}

pub(crate) fn fib(n: i64) -> i64 {
    if (n).le(&1) {
        return n;
    };
    fib(n - 1) + fib(n - 2)
}

pub(crate) fn __main__() {
    println!("{}", &format!("{}", add(2, 3)));
    println!("{}", &format!("{}", fib(10)));
}
```

</details>
