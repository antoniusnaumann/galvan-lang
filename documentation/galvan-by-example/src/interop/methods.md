# Methods and Associated Items

Methods on imported Rust types work with plain member syntax — `.len()`,
`.to_uppercase()`, `.parse()` and friends appear throughout this book.
Beyond that, Galvan defines how to reach *namespaced* methods and
type-associated items:

**Namespaced methods.** Extension methods from another crate are callable
without an import by qualifying the crate:

```galvan
fn main() {
    let book = "content"
    let score = book.reader::read_and_judge()
}
```

Importing the namespace (`use reader`) makes such methods available
unqualified — use it when you want them everywhere; if two imports clash,
fall back to the qualified form.

**Associated functions and constants.** `::` selects namespaces; `.` selects
items that belong to a type — including associated functions and constants:

```galvan
let addr = std::net::SocketAddr.from(([127, 0, 0, 1], 3000))
let created = axum::http::StatusCode.CREATED
let router = Router.new()
```

> [!WARNING]
> **Partially implemented.** Namespaced method calls parse and generate code,
> but are not yet typechecked against the imported signature. Associated
> items with `Type.item` syntax are the design target (they already drive the
> rustdoc lifting of inherent and trait items), but the
> `TypeName.associated_function()` receiver position does not parse yet.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let book: String = format!("content");
    let score: _ = {
        use reader::*;
        book.read_and_judge()
    };
}
```

A namespace-qualified method call becomes a block that imports the crate's
extension traits locally — scoping the import to exactly one call.

</details>
