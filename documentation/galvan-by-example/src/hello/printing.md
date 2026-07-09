# Printing

Galvan has four built-in output functions. They accept any value that can be
formatted:

```galvan
fn main() {
    let dog = "Rex"

    print("no newline. ")
    println("with newline")
    debug(dog)

    if dog != "Rex" {
        panic("expected Rex, found \(dog)")
    }
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let dog: String = format!("Rex");
    print!("{}", &format!("no newline. "));
    println!("{}", &format!("with newline"));
    println!("{:?}", &dog);
    if (dog).ne(&format!("Rex")) {
        panic!("{}", &format!("expected Rex, found {}", dog));
    };
}
```

The four functions map directly onto Rust's `print!`, `println!`,
`println!("{:?}", ..)` and `panic!` macros.

</details>

- `print` writes to standard output without a trailing newline.
- `println` writes a line to standard output.
- `debug` prints the debug representation of a value — handy for structs and
  collections that have no plain text form.
- `panic` aborts the program with a message.

A fifth built-in, `assert`, checks that a condition holds and panics
otherwise. Many examples in this book use it to state their expected results
inline; it is also the backbone of [test blocks](../testing.md).

Like every function called with a single argument in statement position, the
parentheses can be dropped:

```galvan
fn main() {
    println "Hello World!"
}
```
