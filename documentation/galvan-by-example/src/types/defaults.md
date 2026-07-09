# Default Field Values

Struct fields can carry default values. Any field with a default may be
omitted at the construction site:

```galvan
type Book {
    title: String = "Field Notes"
    content: String = "No notes yet"
}

fn main() {
    let blank = Book()
    let packed = Book(content: "Packed lunch at noon")

    println(blank.title)    // Field Notes
    println(packed.content) // Packed lunch at noon
}
```

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Book {
    pub(crate) title: String,
    pub(crate) content: String,
}

pub(crate) fn __main__() {
    let blank: Book = Book {
        title: format!("Field Notes"),
        content: format!("No notes yet"),
    };
    let packed: Book = Book {
        title: format!("Field Notes"),
        content: format!("Packed lunch at noon"),
    };
    println!("{}", blank.title);
    println!("{}", packed.content);
}
```

Defaults are filled in at each construction site, so partial construction has
no runtime cost.

</details>

> [!NOTE]
> When every field can be defaulted, the type is constructible with `Type()` —
> and Galvan can emit a Rust `Default` implementation for it, so the type
> satisfies Rust APIs that expect `Default`.
