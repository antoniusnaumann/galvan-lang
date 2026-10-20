# Receivers: self, mut self, ref self

Member functions declare how they use their receiver, and callers annotate
the receiver the same way they annotate arguments:

```galvan
type Dog {
    name: String
}

type Human {
    name: String
    ref dog: Dog
}

fn rename(mut self: Dog, name: String) {
    self.name = name
}

fn replace(ref self: Dog, replacement: Dog) {
    self = replacement
}

fn share(ref self: Dog, with co_owner_name: String) -> Human {
    Human(name: co_owner_name, dog: ref self)
}

fn main() {
    mut dog = Dog(name: "Milo")
    dog.mut.rename("Scout")

    ref shared_dog = Dog(name: "Rex")
    let human = shared_dog.ref.share(with: "George")
    shared_dog.name = "Lassie"

    assert human.dog.name == "Lassie"
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

    pub(crate) fn share__with(
        __self: std::sync::Arc<std::sync::Mutex<Self>>,
        co_owner_name: &str,
    ) -> Human {
        Human {
            name: co_owner_name.to_owned(),
            dog: ::std::sync::Arc::clone(&__self),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Human {
    pub(crate) name: String,
    pub(crate) dog: std::sync::Arc<std::sync::Mutex<Dog>>,
}

impl PartialEq for Human {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && ::galvan::std::__ref_value_eq(&self.dog, &other.dog)
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
    let human: Human = Dog::share__with((::std::sync::Arc::clone(&shared_dog)), &format!("George"));
    shared_dog.lock().unwrap().name = format!("Lassie");
    assert_eq!(human.dog.lock().unwrap().name, format!("Lassie"),);
}
```

`mut self` is `&mut self`. A `ref self` receiver takes the shared handle
(`Arc<Mutex<Self>>`) directly — which is why replacing the pointed-to value
from inside the method is possible.

Both forms can replace the value during the call, and the caller observes that
replacement. The difference is lifetime: a `mut` borrow ends with the call and
cannot be stored, while a cloned `ref` handle can be retained in another value
and continue sharing state afterward.

</details>

- A plain `self: Dog` receiver reads the value — calls need no annotation.
- `mut self` mutates the receiver: call with `dog.mut.method(...)`,
  `(mut dog).method(...)`, or the free-function form `method(mut dog, ...)`.
- `ref self` takes the receiver as a shared reference, so the method is free
  to store a handle to it in a struct.
