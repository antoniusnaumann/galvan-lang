# Enums

A `type` whose body lists capitalized variants declares an enum. Variants can
be bare, carry unnamed values, or carry named fields:

```galvan
pub type Color {
    Transparent
    Gray(U8)
    Rgb(r: U8, g: U8, b: U8)
}

fn main() {
    let transparent = Color::Transparent
    let gray = Color::Gray(128)
    let rgb = Color::Rgb(r: 100, g: 10, b: 150)

    assert gray == Color::Gray(128)
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

pub(crate) fn __main__() {
    let transparent: Color = Color::Transparent;
    let gray: Color = Color::Gray(128);
    let rgb: Color = Color::Rgb {
        r: 100,
        g: 10,
        b: 150,
    };
    assert_eq!(gray, Color::Gray(128),);
}
```

Galvan enums are Rust enums, one to one — bare variants, tuple variants, and
struct variants.

</details>

- Variants are constructed and referenced with `Type::Variant`.
- Named-field variants use the same named-argument syntax as struct
  construction.
- Enums are inspected with [`match`](../control/match.md).
