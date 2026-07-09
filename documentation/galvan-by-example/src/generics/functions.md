# Generic Functions and where

Functions become generic the same way — use a lowercase type. Trait bounds go
in a `where` clause; several parameters can share one bound:

```galvan
fn str(value: t) -> String where t: ToString {
    value.to_string()
}

fn concat_str(self: a, other: b) -> String where a, b: ToString {
    self.to_string() ++ other.to_string()
}

fn main() {
    assert str(15) == "15"
    assert str("fragile") == "fragile"
    assert "order ".concat_str("ready") == "order ready"
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub trait Generic_A_Ext<A> {
    fn concat_str<B>(&self, other: &B) -> String
    where
        A: ToString,
        B: ToString;
}

impl<A> Generic_A_Ext<A> for A
where
    A: ToString,
{
    fn concat_str<B>(&self, other: &B) -> String
    where
        A: ToString,
        B: ToString,
    {
        [
            (self.to_string()).to_owned(),
            (other.to_string()).to_owned(),
        ]
        .concat()
    }
}

pub(crate) fn str<T>(value: &T) -> String
where
    T: ToString,
{
    value.to_string()
}

pub(crate) fn __main__() {
    assert_eq!(str(&15), format!("15"),);
    assert_eq!(str(&format!("fragile")), format!("fragile"),);
    assert_eq!(
        (&format!("order ")).concat_str(&format!("ready")),
        format!("order ready"),
    );
}
```

A generic free function is a generic Rust function; a generic *method* becomes
a blanket-implemented extension trait.

</details>

- `where t: ToString` — one bound; `where a, b: ToString` — the same bound
  for two parameters.
- A generic `self` parameter turns the function into a method on *every*
  conforming type: `15.repr()`, `"fragile".repr()`.
