# match

`match` inspects a value, destructures enum variants, and returns the value of
the taken branch. Branches are written `pattern { body }`:

```galvan
pub type Color {
    Transparent
    Gray(U8)
    Rgb(r: U8, g: U8, b: U8)
}

fn classify(color: Color) -> String {
    match color {
        Transparent { "transparent" }
        Gray(value) { "gray \(value)" }
        Rgb(r: red, b: blue, g: _) { "rgb \(red) \(blue)" }
        _ { "unknown" }
    }
}

fn main() {
    assert classify(Color::Gray(42)) == "gray 42"
    assert classify(Color::Rgb(r: 10, g: 20, b: 30)) == "rgb 10 30"
}
```

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    Transparent,
    Gray(u8),
    Rgb { r: u8, g: u8, b: u8 },
}

pub(crate) fn classify(color: &Color) -> String {
    match color.to_owned() {
        Color::Transparent => {
            format!("transparent")
        }
        Color::Gray(value) => {
            format!("gray {}", value)
        }
        Color::Rgb {
            r: red,
            g: _,
            b: blue,
        } => {
            format!("rgb {} {}", red, blue)
        }
        _ => {
            format!("unknown")
        }
    }
}

pub(crate) fn __main__() {
    assert_eq!(classify(&Color::Gray(42)), format!("gray 42"),);
    assert_eq!(
        classify(&Color::Rgb {
            r: 10,
            g: 20,
            b: 30
        }),
        format!("rgb 10 30"),
    );
}
```

Patterns expand to fully qualified Rust patterns; the scrutinee is cloned so
that matching never consumes the matched value.

</details>

- Variant names appear without the type prefix inside `match`.
- Named fields can be bound (`r: red`), ignored (`g: _`), and listed in any
  order.
- `_` is the catch-all pattern.
