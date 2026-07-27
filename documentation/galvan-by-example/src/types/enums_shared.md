# Enums with Shared Data

Enums often carry a piece of data that every variant needs. Instead of
repeating the field in each variant, Galvan lets the enum declare **common
fields** in parentheses after its name:

```galvan
pub type Theme(name: String) {
    Plain
    Monochrome(String)
    Contrast(background: String, foreground: String)
}

fn main() {
    let theme = Theme::Contrast(
        name: "Midnight",
        background: "navy",
        foreground: "white",
    )

    assert theme.name == "Midnight"
    assert theme == Theme::Contrast(
        name: "Midnight",
        background: "navy",
        foreground: "white",
    )

    let foreground = match theme {
        Plain { "default" }
        Monochrome(color) { color }
        Contrast(background: _, foreground: color) { color }
    }
    assert foreground == "white"
}
```

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ThemeVariantTag {
    Plain,
    Monochrome(String),
    Contrast {
        background: String,
        foreground: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: String,
    pub(crate) __variant: ThemeVariantTag,
}

pub(crate) fn __main__() {
    let theme: Theme = Theme {
        name: format!("Midnight"),
        __variant: ThemeVariantTag::Contrast {
            background: format!("navy"),
            foreground: format!("white"),
        },
    };
    assert_eq!(theme.name, format!("Midnight"),);
    assert_eq!(
        theme,
        Theme {
            name: format!("Midnight"),
            __variant: ThemeVariantTag::Contrast {
                background: format!("navy"),
                foreground: format!("white")
            }
        },
    );
    let foreground: String = match (theme.to_owned()).__variant {
        ThemeVariantTag::Plain => {
            format!("default")
        }
        ThemeVariantTag::Monochrome(color) => color,
        ThemeVariantTag::Contrast {
            background: _,
            foreground: color,
        } => color,
    };
    assert_eq!(foreground, format!("white"),);
}
```

</details>

The `name` field is shared by every variant; variant-specific fields remain on
their cases. Construction supplies both. Common fields inherit the visibility
of the enum type, just like struct fields.

The generated Rust represents `Theme` as a struct containing `name` and an
internal tag enum. The tag carries the variant-specific fields, keeping the
common data in one place while preserving exhaustive matching. Equality
compares the values of both the common fields and the active variant.
