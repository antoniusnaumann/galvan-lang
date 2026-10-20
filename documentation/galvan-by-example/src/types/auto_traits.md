# Auto Traits and @derive

Galvan derives common traits automatically. Today every struct and enum
derives `Clone`, `Debug`, and `PartialEq` (as the generated Rust throughout
this book shows).

The implemented derive set is visible on every declared type:

```galvan
type SessionToken {
    value: String
}
```

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SessionToken {
    pub(crate) value: String,
}
```

</details>

The full design goes further: `Clone`, `Copy`, `Debug`, `Default`,
`PartialEq`, `Eq`, `Hash`, `serde::Serialize`, and `serde::Deserialize` will
be derived whenever all fields conform, unless the type opts out. Non-auto
traits will be derived explicitly, and libraries will be able to declare
their own auto traits:

```galvan
@derive(!Clone)
type SessionToken {
    value: String
}

@derive(Response)
type HealthResponse {
    status: String
}

auto trait CacheSafe
```

An explicit trait implementation always overrides the derived one.

> [!WARNING]
> **Not implemented yet.** The conditional auto-trait model, `@derive(...)`
> annotations, opt-outs such as `@derive(!Clone)`, and user-declared
> `auto trait`s do not parse yet. What is implemented today is the fixed
> `Clone, Debug, PartialEq` derive on every declared type.
