# Auto Traits and @derive

Galvan derives common traits automatically. Today every struct and enum
derives `Clone`, `Debug`, and `PartialEq` (as the generated Rust throughout
this book shows).

The full design goes further: a set of **auto traits** — `Clone`, `Copy`,
`Debug`, `Default`, `PartialEq`, `Eq`, `Hash`, `serde::Serialize`, and
`serde::Deserialize` — is derived for a type whenever all of its fields
conform, unless the type opts out:

```galvan
@derive(!Clone)
type SessionToken {
    value: String
}
```

Explicit `@derive(...)` documents intended conformance, and libraries can
declare their own auto traits:

```galvan
@derive(Clone, Debug, serde::Serialize)
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

<details>
<summary>Generated Rust (today)</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SessionToken {
    pub(crate) value: String,
}
```

Once auto traits land, this derive list will expand and contract based on
field capabilities and opt-outs — e.g. `serde::Serialize` only when every
field is serializable, and no `Clone` for the example above.

</details>
