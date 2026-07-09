# Parentheses-Free Calls

In statement position, call parentheses are optional — arguments follow the
function name, separated by commas. You have seen this with `print` and
`println` already; it works for any function:

```galvan
fn deliver(order: String, priority: Int) {
    println("Delivering \(order) with priority \(priority)")
}

fn main() {
    deliver "Coffee", 1
    println "done"
}
```

Two boundaries keep the syntax unambiguous:

- Calls with **no** arguments always need parentheses — `report()` — since a
  bare name is a variable reference.
- Paren-free calls cannot nest inside other call arguments.

> [!WARNING]
> The design also allows paren-free calls on the right-hand side of an
> assignment (`let result = add 2, 3`), but that position does not parse
> yet — today the syntax works in statement position and for
> [trailing closures](../closures/trailing.md) in member chains.

<details>
<summary>Generated Rust</summary>

```rust
pub(crate) fn deliver(order: &str, priority: i64) {
    {
        println!(
            "{}",
            &format!("Delivering {} with priority {}", order, priority)
        );
    };
}

pub(crate) fn __main__() {
    deliver(&format!("Coffee"), 1);
    println!("{}", &format!("done"));
}
```

Purely syntactic sugar — the generated call is identical to the
parenthesized form.

</details>
