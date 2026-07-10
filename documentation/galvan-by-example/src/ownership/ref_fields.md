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

#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    pub(crate) name: String,
    pub(crate) dog: std::sync::Arc<std::sync::Mutex<Dog>>,
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
    println!("{}", &format!("{}", person.dog.age));
    println!("{}", &format!("{}", dog.lock().unwrap().age));
}
```

> [!WARNING]
> `ref` fields are usable but their codegen still has known gaps: reading a
> `ref` field through a chain (`person.dog.age` above misses its lock) emits
> Rust that does not compile yet, derives such as `PartialEq` are generated
> even though `Arc<Mutex<T>>` does not support them, and shared primitive
> fields do not use atomics yet. The single-handle patterns on this page's
> `dog` variable are covered by tests.

</details>

The person's `dog` and the local `dog` are the same animal: mutating one is
visible through the other. Omitting the `ref` is a compiler error, similar to how forgetting a `mut` for mutable parameters fails.
