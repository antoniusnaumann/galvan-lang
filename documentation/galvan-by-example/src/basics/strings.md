# Strings and Characters

Strings are owned, growable UTF-8 text. Concatenation uses `++`, so `+` is only used for addition. Characters use single quotes and concatenate onto
strings the same way:

```galvan
fn main() {
    let greeting = "Hello" ++ " " ++ "World"

    mut cheer = "Go"
    cheer ++= " Galvan"
    cheer ++= '!'

    let newline = '\n'
    let quote = '\''
    let tab: Char = '\t'
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let greeting: String = format!(
        "{}{}",
        format!("{}{}", format!("Hello"), format!(" ")),
        format!("World")
    );
    let mut cheer: String = format!("Go");
    cheer.push_str(&format!(" Galvan"));
    cheer.push('!');
    let newline: char = '\n';
    let quote: char = '\'';
    let tab: char = '\t';
}
```

`++` on strings lowers to `format!` concatenation, `++=` to `push_str`, or
`push` when the right-hand side is a `Char`.

</details>

- `++` builds a new string; `++=` appends in place and requires a `mut`
  binding.
- Character escapes cover the usual suspects: `'\n'`, `'\t'`, `'\\'`, `'\''`.
- String escapes and interpolation are covered in
  [String Interpolation](../hello/interpolation.md).
