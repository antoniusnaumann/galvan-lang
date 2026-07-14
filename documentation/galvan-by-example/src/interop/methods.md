# Methods and Associated Items

Methods on imported Rust types work with plain member syntax — `.len()`,
`.to_uppercase()`, `.parse()` and friends appear throughout this book.
Beyond that, Galvan defines how to reach *namespaced* methods and
type-associated items:

**Namespaced methods.** Extension methods from another crate, i.e., methods that are defined in a crate that does not define the type itself, are callable
without an import by qualifying the crate:

```galvan
fn main() {
    let book = "content"
    let score = book.reader::read_and_judge()
}
```

<!-- galvan-book: rustdoc-dependent -->

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let book: String = format!("content");
    let score: _ = {
        use reader::String_Ext;
        book.read_and_judge()
    };
}
```

A namespace-qualified method call becomes a block that imports the matching
`{TypeName}_Ext` trait locally — the same trait shape generated for Galvan
extension methods. These recognized extension traits participate in
typechecking; methods from unrelated trait implementations are not exposed as
extension methods.

</details>

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
> Associated functions and constants parse and typecheck when rustdoc metadata
> identifies their receiver. Generic builder APIs can still require type
> information that the interop layer does not yet propagate through a chain.
