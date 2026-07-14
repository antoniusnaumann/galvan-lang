# Enums with Shared Data

> [!WARNING]
> **Not implemented yet.** Shared variant data does not parse yet — the
> transpiler rejects the declaration below. This page documents the intended
> design.

Enums often carry a piece of data that every variant needs. Instead of
repeating the field in each variant, Galvan lets the enum declare **common
fields** in parentheses after its name:

```galvan
pub type Theme(name: String) {
    Plain
    Monochrome(Color)
    Dark(background: Color, foreground: Color)
    Light(background: Color, foreground: Color)
}
```

The planned Rust lowering represents `Theme` as a struct containing `name` and
a private tag enum. That tag enum carries the variant-specific fields, keeping
the common data in one place while preserving exhaustive matching.

The `name` field is shared by all variants; variant-specific fields are
declared on each case. Construction supplies both:

```galvan
let theme = Theme::Dark(
    name: "Midnight",
    background: Color(r: 0, g: 0, b: 32),
    foreground: Color(r: 255, g: 255, b: 255),
)
```
