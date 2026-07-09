# Generic Types

A struct field with a lowercase type makes the whole type generic. Member
functions name the parameter in the receiver type:

```galvan
type Container {
    value: t
}

fn get_value(self: Container<t>) -> t {
    self.value
}

fn main() {
    let shipment = Container(value: 42)

    assert shipment.value == 42
    assert shipment.get_value() == 42
}
```

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Container<T: ToOwned<Owned = T>> {
    pub(crate) value: T,
}

impl<T: ToOwned<Owned = T>> Container<T> {
    pub(crate) fn get_value(&self) -> T {
        self.value.to_owned()
    }
}

pub(crate) fn __main__() {
    let shipment: Container<_> = Container { value: 42 };
    assert_eq!(shipment.value, 42,);
    assert_eq!((&shipment).get_value(), 42,);
}
```

Type parameters are uppercased for Rust. The `ToOwned<Owned = T>` bound is
Galvan's value-semantics contract: every generic value must be clonable to an
owned form.

</details>

No `<t>` is needed on the declaration — using `t` in a field *is* the
declaration. Construction infers the parameter from the argument.
