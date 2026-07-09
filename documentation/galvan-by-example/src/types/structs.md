# Structs

A `type` with named fields declares a struct. Instances are created by calling
the type name with **named arguments** — there is no separate constructor
syntax and no builder boilerplate:

```galvan
pub type Dog {
    name: String
    age: Int
}

fn main() {
    mut dog = Dog(name: "Rex", age: 3)

    dog.age = 4

    println("\(dog.name) is \(dog.age)")
}
```

- Fields are separated by newlines or commas.
- `pub type` exports the type; the fields themselves stay crate-internal.
- Field access and assignment use `.`; assigning a field requires the binding
  to be `mut` (or `ref` — see [Ownership](../ownership/index.md)).

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Dog {
    pub(crate) name: String,
    pub(crate) age: i64,
}

pub(crate) fn __main__() {
    let mut dog: Dog = Dog {
        name: format!("Rex"),
        age: 3,
    };
    dog.age = 4;
    println!("{}", &format!("{} is {}", dog.name, dog.age));
}
```

Construction is a plain struct literal. `Clone`, `Debug`, and `PartialEq` are
derived automatically — see [Auto Traits](auto_traits.md).

</details>
