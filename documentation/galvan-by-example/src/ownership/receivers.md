# Receivers: self, mut self, ref self

Member functions declare how they use their receiver, and callers annotate
the receiver the same way they annotate arguments:

```galvan
type Dog {
    name: String
}

fn rename(mut self: Dog, name: String) {
    self.name = name
}

fn replace(ref self: Dog, replacement: Dog) {
    self = replacement
}

fn main() {
    mut dog = Dog(name: "Milo")
    dog.mut.rename("Scout")

    ref shared_dog = Dog(name: "Rex")
    shared_dog.ref.replace(Dog(name: "Lassie"))
}
```

- A plain `self: Dog` receiver reads the value — calls need no annotation.
- `mut self` mutates the receiver: call with `dog.mut.method(...)`,
  `(mut dog).method(...)`, or the free-function form `method(mut dog, ...)`.
- `ref self` takes the receiver as a shared reference — the method can even
  replace the referenced value itself.

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Dog {
    pub(crate) name: String,
}

impl Dog {
    pub(crate) fn rename(&mut self, name: &str) {
        {
            self.name = name.to_owned();
        };
    }

    pub(crate) fn replace(__self: std::sync::Arc<std::sync::Mutex<Self>>, replacement: &Dog) {
        {
            *__self.lock().unwrap() = replacement.to_owned();
        };
    }
}

pub(crate) fn __main__() {
    let mut dog: Dog = Dog {
        name: format!("Milo"),
    };
    (&mut dog).rename(&format!("Scout"));
    let mut shared_dog: std::sync::Arc<std::sync::Mutex<Dog>> = (&(Dog {
        name: format!("Rex"),
    }))
        .__to_ref();
    Dog::replace(
        (::std::sync::Arc::clone(&shared_dog)),
        &Dog {
            name: format!("Lassie"),
        },
    );
}
```

`mut self` is `&mut self`. A `ref self` receiver takes the shared handle
(`Arc<Mutex<Self>>`) directly — which is why replacing the pointed-to value
from inside the method is possible.

</details>
