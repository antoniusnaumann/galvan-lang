# Hello World

Galvan programs start at a regular `main` function. There are no semicolons at
line ends and no parentheses gymnastics — `fn main` and one statement is a
complete program:

```galvan
fn main() {
    print("Hello World!")
}
```

`main` can optionally receive the process argument vector. The first element
is the executable name, just like `std::env::args()` in Rust:

```galvan
fn main(args: [String]) {
    for args |arg| {
        println(arg)
    }
}
```

`[String]` is Galvan's array type — collections get their own
[chapter](../collections/index.md) later.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    print!("{}", &format!("Hello World!"));
}
```

With arguments:

```rust
pub(crate) fn __main__() {
    let args: ::std::vec::Vec<String> = ::std::env::args().collect();
    {
        for arg in args {
            println!("{}", arg);
        }
    };
}
```

The generated `__main__` is called from a `fn main` that `galvan::main!()`
expands to in `src/main.rs`.

</details>
