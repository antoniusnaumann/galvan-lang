# Variables and Mutability

`let` introduces an immutable binding, `mut` a mutable one. Types are inferred
but can be annotated with `: Type`. Bindings can be shadowed by a new `let`,
even with a different type:

```galvan
fn main() {
    let price = 20
    mut count = 1

    count += 1

    let label = "total"
    let label = "\(label): \(price * count)"

    println(label)
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let price: _ = 20;
    let mut count: _ = 1;
    count += 1;
    let label: String = format!("total");
    let label: String = format!("{}: {}", label, price * count);
    println!("{}", &label);
}
```

</details>

Assigning a variable to another variable **copies** the value. There is no
move semantics at the surface level — the original stays usable and the copy
is independent:

```galvan
fn main() {
    mut original = "Bello"
    mut copy = original

    original = "Hasso"

    println(original) // Hasso
    println(copy)     // Bello
}
```

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn __main__() {
    let mut original: String = format!("Bello");
    let mut copy: String = original.to_owned();
    original = format!("Hasso");
    println!("{}", &original);
    println!("{}", &copy);
}
```

Galvan inserts `.to_owned()` where Rust would otherwise move: assignment in
Galvan is a semantic copy, and the transpiler decides between borrowing and
cloning so that user code never has to.

</details>
