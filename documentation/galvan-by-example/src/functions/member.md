# Member Functions

All functions are declared top-level. If the first parameter is named `self`,
the function can be called with method syntax on that type:

```galvan
type Dog {
    name: String
}

fn bark(self: Dog) {
    println("\(self.name) barks")
}

fn main() {
    let dog = Dog(name: "Milo")
    dog.bark()
    bark(dog)
}
```

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Dog {
    pub(crate) name: String,
}

impl Dog {
    pub(crate) fn bark(&self) {
        {
            println!("{}", &format!("{} barks", self.name));
        };
    }
}

pub(crate) fn __main__() {
    let dog: Dog = Dog {
        name: format!("Milo"),
    };
    (&dog).bark();
    (&dog).bark();
}
```

Member functions for a local type are collected into an `impl` block. Both
call spellings lower to the same method call; because `self: Dog` passes by
value (see [Pass by Value](../ownership/by_value.md)), the receiver is
borrowed rather than consumed.

</details>

- `bark(dog)` resolves to `dog.bark()` if no matching function with the same signature exists.
- Mutating receivers (`mut self`) and reference receivers (`ref self`) are
  covered in [Receivers](../ownership/receivers.md).
