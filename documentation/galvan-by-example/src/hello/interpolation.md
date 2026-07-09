# String Interpolation

Strings interpolate values with `\(...)`. Any expression can appear inside the
parentheses — variables, arithmetic, member access, or function calls:

```galvan
fn main() {
    let x = 3
    let y = 7

    println("\(x) + \(y) = \(x + y)")
    println("literal braces: {\(x + y)}")
    println("escaped quotes: \"quoted\"")
    println("unicode escape: \u{1F600}")
}
```

- `{` and `}` are ordinary characters in Galvan strings; no doubling is
  required.
- `\"` escapes a quote, `\u{...}` writes a Unicode scalar value.
- Interpolation also reaches into fields: `"Hi, \(dog.name)!"`.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let x: _ = 3;
    let y: _ = 7;
    println!("{}", &format!("{} + {} = {}", x, y, x + y));
    println!("{}", &format!("literal braces: {{{}}}", x + y));
    println!("{}", &format!("escaped quotes: \"quoted\""));
    println!("{}", &format!("unicode escape: \u{1F600}"));
}
```

Interpolated strings become `format!` calls; literal braces are doubled for
Rust's formatter. The `let x: _ = 3` annotation leaves the concrete integer
type to Rust's inference.

</details>
