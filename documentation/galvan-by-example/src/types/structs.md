# Structs

A `type` with named fields declares a struct. Instances are created by calling
the type name with **named arguments**.

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

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Dog {
    pub name: String,
    pub age: i64,
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

Note that `Clone`, `Debug`, and `PartialEq` are
derived automatically whenever possible — see [Auto Traits](auto_traits.md).

</details>

- Fields are separated by newlines or commas.
- `pub type` exports the type; the fields themselves follow the type visibility, i.e., fields of public structs are public by default.
- Field access and assignment use `.`; assigning a field requires the binding
  to be `mut` (or `ref` — see [Ownership](../ownership/index.md)).
