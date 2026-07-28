# refs in Structs

Struct fields can be declared `ref` to share state between the struct and the
outside world. Construction requires the same explicit `ref` annotation as a
call:

```galvan
type Dog {
    name: String
    age: Int
}

pub type Person {
    name: String
    ref dog: Dog
}

fn main() {
    ref dog = Dog(name: "Milo", age: 5)
    let person = Person(name: "Jochen", dog: ref dog)

    dog.age += 1

    println("\(person.dog.age)") // 6
    println("\(dog.age)")        // 6
}
```

<details>
<summary>Generated Rust</summary>

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Dog {
    pub(crate) name: String,
    pub(crate) age: i64,
}

#[derive(Clone, Debug)]
pub struct Person {
    pub name: String,
    pub dog: std::sync::Arc<std::sync::Mutex<Dog>>,
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && ::galvan::std::__ref_value_eq(&self.dog, &other.dog)
    }
}

pub(crate) fn __main__() {
    let mut dog: std::sync::Arc<std::sync::Mutex<Dog>> = (&(Dog {
        name: format!("Milo"),
        age: 5,
    }))
        .__to_ref();
    let person: Person = Person {
        name: format!("Jochen"),
        dog: ::std::sync::Arc::clone(&dog),
    };
    dog.lock().unwrap().age += 1;
    println!("{}", &format!("{}", person.dog.lock().unwrap().age));
    println!("{}", &format!("{}", dog.lock().unwrap().age));
}
```

</details>

Chained reads through `ref` fields take the field's lock automatically.
`PartialEq` compares the values stored behind `ref` fields. Locks are acquired
in a stable order, and aliases of the same mutex compare immediately.

The person's `dog` and the local `dog` are the same animal: mutating one is
visible through the other. Omitting the `ref` is a compiler error, similar to how forgetting a `mut` for mutable parameters fails.
